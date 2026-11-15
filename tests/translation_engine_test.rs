//! Integration tests for the subtitle translation engine.
//!
//! Covers UUIDv7 cue ID guarantees, glossary override precedence, batching
//! and ordering, missing-cue retry, and fallback behavior when the AI response
//! keeps omitting a cue ID. These tests exercise the engine without any real network access
//! by feeding deterministic responses through an in-process `AIProvider`
//! double that captures cue IDs from the prompt and rewrites placeholder
//! `__ID<N>__` tokens in the canned response.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use subx_core::Result;
use subx_core::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_core::core::report::{ProgressEvent, Reporter, TranslationPass, TranslationProgress};
use subx_core::core::translation::{
    GlossaryEntry, TranslationEngine, TranslationRequest, generate_cue_ids,
};
use subx_core::error::SubXError;
use subx_core::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use uuid::Uuid;

/// Extract the 48-bit `unix_time_ts` field from a UUIDv7 value.
fn unix_time_ms(id: &Uuid) -> u64 {
    let bytes = id.as_bytes();
    ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | (bytes[5] as u64)
}

/// Test double that returns canned responses in order. When the canned text
/// contains `__ID<N>__` placeholders, those are substituted with cue IDs
/// extracted from the most recent translation prompt so the response stays
/// in sync with the engine-generated UUIDv7 values.
struct PlaceholderAI {
    responses: Mutex<Vec<String>>,
}

impl PlaceholderAI {
    fn new(responses: Vec<&str>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into_iter().map(String::from).collect()),
        })
    }
}

#[async_trait]
impl AIProvider for PlaceholderAI {
    async fn analyze_content(&self, _r: AnalysisRequest) -> Result<MatchResult> {
        unreachable!("translation tests do not call analyze_content")
    }

    async fn verify_match(&self, _r: VerificationRequest) -> Result<ConfidenceScore> {
        unreachable!("translation tests do not call verify_match")
    }

    async fn chat_completion(&self, messages: Vec<serde_json::Value>) -> Result<String> {
        let prompt = messages
            .last()
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let mut response = {
            let mut responses = self.responses.lock().unwrap();
            assert!(
                !responses.is_empty(),
                "PlaceholderAI ran out of canned responses"
            );
            responses.remove(0)
        };

        if response.contains("__ID") {
            let ids: Vec<String> = prompt
                .lines()
                .filter_map(|l| l.trim().strip_prefix("- id: "))
                .map(|s| s.trim().to_string())
                .collect();
            for (i, id) in ids.iter().enumerate() {
                response = response.replace(&format!("__ID{}__", i), id);
            }
        }
        if response.contains("__FMT") {
            let placeholders = formatting_placeholders(&prompt);
            for (i, placeholder) in placeholders.iter().enumerate() {
                response = response.replace(&format!("__FMT{}__", i), placeholder);
            }
        }
        Ok(response)
    }
}

fn formatting_placeholders(prompt: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut search_start = 0usize;
    while let Some(start_rel) = prompt[search_start..].find("__SUBX_FMT_") {
        let start = search_start + start_rel;
        let after_prefix = start + 2;
        let Some(end_rel) = prompt[after_prefix..].find("__") else {
            break;
        };
        let end = after_prefix + end_rel + 2;
        placeholders.push(prompt[start..end].to_string());
        search_start = end;
    }
    placeholders
}

fn make_subtitle(texts: &[&str]) -> Subtitle {
    let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
    let mut sub = Subtitle::new(SubtitleFormatType::Srt, metadata);
    for (i, text) in texts.iter().enumerate() {
        let start = Duration::from_secs((i as u64) * 2 + 1);
        let end = Duration::from_secs((i as u64) * 2 + 2);
        sub.entries
            .push(SubtitleEntry::new(i + 1, start, end, text.to_string()));
    }
    sub
}

#[test]
fn cue_ids_are_uuidv7_with_strictly_increasing_unix_timestamps() {
    let ids = generate_cue_ids(5);
    assert_eq!(ids.len(), 5);
    let mut last_ts = 0u64;
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(id.get_version_num(), 7, "cue id #{i} must be UUIDv7: {id}");
        let ts = unix_time_ms(id);
        if i > 0 {
            assert!(
                ts > last_ts,
                "cue id #{i} unix_time_ts must be strictly greater (got {ts} after {last_ts})"
            );
            assert!(
                ts - last_ts >= 1,
                "cue ids must be spaced by at least 1ms (delta = {})",
                ts - last_ts
            );
        }
        last_ts = ts;
    }
}

#[tokio::test]
async fn translation_engine_preserves_order_and_terminology() {
    let term_resp = r#"{"terms":[{"source":"Alice","target":"愛麗絲"}]}"#;
    let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"哈囉 愛麗絲"},{"id":"__ID1__","text":"再見 愛麗絲"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);

    let engine = TranslationEngine::new(provider, 10).expect("batch_size > 0");
    let subtitle = make_subtitle(&["Hello Alice", "Goodbye Alice"]);

    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: Some("en".to_string()),
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();

    assert_eq!(result.subtitle.entries.len(), 2);
    assert_eq!(result.subtitle.entries[0].text, "哈囉 愛麗絲");
    assert_eq!(result.subtitle.entries[1].text, "再見 愛麗絲");
    // Timing is preserved.
    assert_eq!(
        result.subtitle.entries[0].start_time,
        Duration::from_secs(1)
    );
    assert_eq!(result.subtitle.entries[0].end_time, Duration::from_secs(2));
    assert_eq!(
        result.subtitle.entries[1].start_time,
        Duration::from_secs(3)
    );
    assert_eq!(result.subtitle.entries[1].end_time, Duration::from_secs(4));
    // Terminology recorded in outcome.
    assert_eq!(result.outcome.translated_cue_count, 2);
    assert_eq!(result.outcome.batch_count, 1);
    assert_eq!(
        result.outcome.effective_terminology.get("Alice").unwrap(),
        "愛麗絲"
    );
}

#[tokio::test]
async fn translation_engine_glossary_overrides_generated_terms() {
    // Generated map suggests Alice -> 愛麗絲 but the user glossary mandates
    // Alice -> 艾莉絲. The effective terminology in the outcome must use the
    // glossary value.
    let term_resp = r#"{"terms":[{"source":"Alice","target":"愛麗絲"}]}"#;
    let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"哈囉 艾莉絲"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);

    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["Hello Alice"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: None,
        glossary_text: Some("Alice = 艾莉絲".to_string()),
        context: None,
        glossary_entries: vec![GlossaryEntry {
            source: "Alice".to_string(),
            target: "艾莉絲".to_string(),
        }],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    assert_eq!(
        result.outcome.effective_terminology.get("Alice").unwrap(),
        "艾莉絲",
        "glossary entry must override AI-generated terminology"
    );
    assert_eq!(result.subtitle.entries[0].text, "哈囉 艾莉絲");
}

#[tokio::test]
async fn translation_engine_splits_into_multiple_batches_in_order() {
    // 3 cues with batch size 2 -> 1 terminology pass + 2 translation
    // batches. The placeholder substitution keeps each batch's IDs aligned
    // with the engine-generated values.
    let term_resp = r#"{"terms":[]}"#;
    let batch1 = r#"{"translations":[{"id":"__ID0__","text":"一"},{"id":"__ID1__","text":"二"}]}"#;
    let batch2 = r#"{"translations":[{"id":"__ID0__","text":"三"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, batch1, batch2]);

    let engine = TranslationEngine::new(provider, 2).unwrap();
    let subtitle = make_subtitle(&["one", "two", "three"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    assert_eq!(result.outcome.batch_count, 2);
    assert_eq!(result.outcome.translated_cue_count, 3);
    let texts: Vec<&str> = result
        .subtitle
        .entries
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(texts, vec!["一", "二", "三"]);
}

#[tokio::test]
async fn translation_engine_retries_missing_cue_after_initial_batches() {
    // The first initial batch omits the second cue. The engine must finish the
    // second initial batch before retrying the missing cue once.
    let term_resp = r#"{"terms":[]}"#;
    let batch1 = r#"{"translations":[{"id":"__ID0__","text":"一"}]}"#;
    let batch2 = r#"{"translations":[{"id":"__ID0__","text":"三"}]}"#;
    let retry = r#"{"translations":[{"id":"__ID0__","text":"二補"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, batch1, batch2, retry]);

    let engine = TranslationEngine::new(provider, 2).unwrap();
    let subtitle = make_subtitle(&["one", "two", "three"]);

    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    let texts: Vec<&str> = result
        .subtitle
        .entries
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(texts, vec!["一", "二補", "三"]);
    assert_eq!(
        result.outcome.batch_count, 3,
        "two initial batches plus one retry request"
    );
}

#[tokio::test]
async fn translation_engine_fills_empty_when_retry_still_omits_cue() {
    let term_resp = r#"{"terms":[]}"#;
    let batch = r#"{"translations":[{"id":"__ID0__","text":"一"}]}"#;
    let retry = r#"{"translations":[]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, batch, retry]);

    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["one", "{\\i1}two{\\i0}"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    assert_eq!(result.subtitle.entries[0].text, "一");
    assert_eq!(
        result.subtitle.entries[1].text, "",
        "still-missing retry cue falls back to empty text even when source had formatting"
    );
    assert_eq!(result.outcome.batch_count, 2);
}

#[tokio::test]
async fn translation_engine_discards_unknown_cue_batch_and_retries_once() {
    let term_resp = r#"{"terms":[]}"#;
    // The first batch response includes one valid cue plus one hallucinated cue.
    // The valid cue from that response must be discarded with the rest of the
    // batch, so the final output must use the retry text.
    let hallucinated = r#"{"translations":[{"id":"__ID0__","text":"discard-me"},{"id":"01900000-9999-7000-8000-000000000000","text":"?"}]}"#;
    let retry =
        r#"{"translations":[{"id":"__ID0__","text":"一補"},{"id":"__ID1__","text":"二補"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, hallucinated, retry]);

    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["one", "two"]);
    let request = TranslationRequest {
        target_language: "ja".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    let texts: Vec<&str> = result
        .subtitle
        .entries
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(texts, vec!["一補", "二補"]);
    assert_eq!(
        result.outcome.batch_count, 2,
        "initial hallucinated response plus one successful retry"
    );
}

#[tokio::test]
async fn translation_engine_fails_when_unknown_cue_id_repeats_after_retry() {
    let term_resp = r#"{"terms":[]}"#;
    let hallucinated =
        r#"{"translations":[{"id":"01900000-9999-7000-8000-000000000000","text":"?"}]}"#;
    let retry_hallucinated =
        r#"{"translations":[{"id":"01900000-9999-7000-8000-000000000001","text":"?"}]}"#;
    let provider: Arc<dyn AIProvider> =
        PlaceholderAI::new(vec![term_resp, hallucinated, retry_hallucinated]);

    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["only cue"]);
    let request = TranslationRequest {
        target_language: "ja".to_string(),
        source_language: None,
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let err = engine
        .translate_subtitle(subtitle, &request)
        .await
        .expect_err("repeated unknown cue id must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown cue ID after retry"),
        "unexpected: {msg}"
    );
}

#[tokio::test]
async fn translation_engine_preserves_supported_format_metadata_for_all_formats() {
    let cases = [
        (
            "srt",
            "1\n00:00:01,000 --> 00:00:02,000\nHello\n",
            SubtitleFormatType::Srt,
            Duration::from_secs(1),
            Duration::from_secs(2),
        ),
        (
            "vtt",
            "WEBVTT\n\n1\n00:00:01.000 --> 00:00:02.000\nHello\n",
            SubtitleFormatType::Vtt,
            Duration::from_secs(1),
            Duration::from_secs(2),
        ),
        (
            "sub",
            "{25}{50}Hello\n",
            SubtitleFormatType::Sub,
            Duration::from_secs(1),
            Duration::from_secs(2),
        ),
        (
            "ass",
            "[Script Info]\nScriptType: v4.00+\n\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:01.00,0:00:02.00,Default,,0000,0000,0000,,Hello\n",
            SubtitleFormatType::Ass,
            Duration::from_secs(1),
            Duration::from_secs(2),
        ),
    ];

    for (name, content, format, start, end) in cases {
        let term_resp = r#"{"terms":[]}"#;
        let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"翻譯後"}]}"#;
        let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);
        let engine = TranslationEngine::new(provider, 10).unwrap();
        let request = TranslationRequest {
            target_language: "zh-TW".to_string(),
            source_language: None,
            glossary_text: None,
            context: None,
            glossary_entries: vec![],
        };

        let result = engine
            .translate_content(content, &request)
            .await
            .unwrap_or_else(|e| panic!("{name} should translate: {e}"));
        assert_eq!(result.subtitle.format, format, "{name} format preserved");
        assert_eq!(result.subtitle.entries.len(), 1, "{name} cue count");
        let entry = &result.subtitle.entries[0];
        assert_eq!(entry.start_time, start, "{name} start time");
        assert_eq!(entry.end_time, end, "{name} end time");
        assert_eq!(entry.text, "翻譯後", "{name} visible text translated");
    }
}

#[tokio::test]
async fn translation_engine_preserves_inline_formatting_tokens() {
    let term_resp = r#"{"terms":[]}"#;
    let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"__FMT0__你好 愛麗絲__FMT1__"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);
    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["<i>Hello Alice</i>"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: Some("en".to_string()),
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let result = engine.translate_subtitle(subtitle, &request).await.unwrap();
    assert_eq!(result.subtitle.entries[0].text, "<i>你好 愛麗絲</i>");
}

#[tokio::test]
async fn translation_engine_rejects_missing_formatting_placeholder() {
    let term_resp = r#"{"terms":[]}"#;
    let cues_resp = r#"{"translations":[{"id":"__ID0__","text":"你好 愛麗絲"}]}"#;
    let provider: Arc<dyn AIProvider> = PlaceholderAI::new(vec![term_resp, cues_resp]);
    let engine = TranslationEngine::new(provider, 10).unwrap();
    let subtitle = make_subtitle(&["{\\i1}Hello Alice{\\i0}"]);
    let request = TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: Some("en".to_string()),
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    };

    let err = engine
        .translate_subtitle(subtitle, &request)
        .await
        .expect_err("missing formatting placeholder must fail");
    assert!(err.to_string().contains("formatting placeholder"));
}

/// One entry in a shared chronology: a progress event the reporter received,
/// or a provider request logged at the moment it was issued. Sharing one log
/// is what lets a test prove the provider's terminology request fell between
/// `TerminologyStarted` and `TerminologyFinished`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Observed {
    Message(String),
    Started,
    Advanced,
    Finished,
    Translation(TranslationProgress),
    Provider(&'static str),
}

/// Records every progress event; the scripted provider appends to the same
/// log so request/event interleaving is observable.
#[derive(Default)]
struct RecordingReporter {
    observations: Arc<Mutex<Vec<Observed>>>,
}

impl RecordingReporter {
    fn observations(&self) -> Vec<Observed> {
        self.observations.lock().unwrap().clone()
    }

    fn translations(&self) -> Vec<TranslationProgress> {
        self.observations()
            .into_iter()
            .filter_map(|observed| match observed {
                Observed::Translation(progress) => Some(progress),
                _ => None,
            })
            .collect()
    }

    fn messages(&self) -> Vec<String> {
        self.observations()
            .into_iter()
            .filter_map(|observed| match observed {
                Observed::Message(message) => Some(message),
                _ => None,
            })
            .collect()
    }
}

impl Reporter for RecordingReporter {
    fn progress(&self, event: &ProgressEvent<'_>) {
        // All five current variants are named; the wildcard is required from
        // outside the defining crate because the enum is `#[non_exhaustive]`.
        #[allow(unreachable_patterns)]
        let observed = match event {
            ProgressEvent::Message(message) => Observed::Message((*message).to_string()),
            ProgressEvent::Started { .. } => Observed::Started,
            ProgressEvent::Advanced { .. } => Observed::Advanced,
            ProgressEvent::Finished { .. } => Observed::Finished,
            ProgressEvent::Translation(progress) => Observed::Translation(*progress),
            _ => Observed::Finished,
        };
        self.observations.lock().unwrap().push(observed);
    }
}

/// What the scripted provider should answer next.
enum Step {
    Terminology(Vec<(&'static str, &'static str)>),
    TranslateAll,
    /// Return every prompt cue except the last `n`.
    TranslateOmitLast(usize),
    /// Return every prompt cue plus one cue ID that was never requested.
    TranslateWithUnknownId,
    Fail(&'static str),
}

fn no_terms() -> Step {
    Step::Terminology(Vec::new())
}

struct ScriptedAI {
    steps: Mutex<VecDeque<Step>>,
    observations: Arc<Mutex<Vec<Observed>>>,
}

impl ScriptedAI {
    fn new(steps: Vec<Step>, observations: Arc<Mutex<Vec<Observed>>>) -> Arc<Self> {
        Arc::new(Self {
            steps: Mutex::new(steps.into()),
            observations,
        })
    }

    fn observe(&self, kind: &'static str) {
        self.observations
            .lock()
            .unwrap()
            .push(Observed::Provider(kind));
    }
}

/// Cue IDs listed in the last user prompt, in prompt order.
fn prompt_cue_ids(prompt: &str) -> Vec<String> {
    prompt
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- id: "))
        .map(|id| id.trim().to_string())
        .collect()
}

fn translations_response(ids: &[String], keep: usize) -> String {
    let entries: Vec<_> = ids
        .iter()
        .take(keep)
        .enumerate()
        .map(|(index, id)| json!({"id": id, "text": format!("text-{index}")}))
        .collect();
    json!({ "translations": entries }).to_string()
}

#[async_trait]
impl AIProvider for ScriptedAI {
    async fn analyze_content(&self, _r: AnalysisRequest) -> Result<MatchResult> {
        unreachable!("progress tests do not call analyze_content")
    }

    async fn verify_match(&self, _r: VerificationRequest) -> Result<ConfidenceScore> {
        unreachable!("progress tests do not call verify_match")
    }

    async fn chat_completion(&self, messages: Vec<serde_json::Value>) -> Result<String> {
        let step = {
            let mut steps = self.steps.lock().unwrap();
            steps
                .pop_front()
                .expect("ScriptedAI ran out of scripted steps")
        };
        let prompt = messages
            .last()
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .unwrap_or("");

        match step {
            Step::Terminology(terms) => {
                self.observe("terminology");
                let terms: Vec<_> = terms
                    .iter()
                    .map(|(source, target)| json!({"source": source, "target": target}))
                    .collect();
                Ok(json!({ "terms": terms }).to_string())
            }
            Step::TranslateAll => {
                self.observe("translation");
                let ids = prompt_cue_ids(prompt);
                Ok(translations_response(&ids, ids.len()))
            }
            Step::TranslateOmitLast(omit) => {
                self.observe("translation");
                let ids = prompt_cue_ids(prompt);
                let keep = ids.len().saturating_sub(omit);
                Ok(translations_response(&ids, keep))
            }
            Step::TranslateWithUnknownId => {
                self.observe("translation");
                let mut entries: Vec<_> = prompt_cue_ids(prompt)
                    .iter()
                    .enumerate()
                    .map(|(index, id)| json!({"id": id, "text": format!("text-{index}")}))
                    .collect();
                entries.push(json!({
                    "id": "01900000-9999-7000-8000-000000000000",
                    "text": "?"
                }));
                Ok(json!({ "translations": entries }).to_string())
            }
            Step::Fail(message) => {
                self.observe("failure");
                Err(SubXError::ai_service(message))
            }
        }
    }
}

fn progress_request() -> TranslationRequest {
    TranslationRequest {
        target_language: "zh-TW".to_string(),
        source_language: Some("en".to_string()),
        glossary_text: None,
        context: None,
        glossary_entries: vec![],
    }
}

fn subtitle_with_cues(count: usize) -> Subtitle {
    let texts: Vec<String> = (0..count).map(|index| format!("cue {index}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    make_subtitle(&refs)
}

fn engine_with(
    reporter: &Arc<RecordingReporter>,
    batch_size: usize,
    steps: Vec<Step>,
) -> TranslationEngine {
    let provider: Arc<dyn AIProvider> = ScriptedAI::new(steps, Arc::clone(&reporter.observations));
    let reporter: Arc<dyn Reporter> = reporter.clone();
    TranslationEngine::new(provider, batch_size)
        .expect("batch_size is non-zero")
        .with_reporter(reporter)
}

const RETRY_NOTICE: &str = "⚠ Translation response contained an unknown cue ID; discarding the batch response and retrying once.";

#[tokio::test]
async fn translation_progress_brackets_terminology_before_glossary_merge() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(
        &reporter,
        10,
        vec![
            Step::Terminology(vec![("Alice", "愛麗絲"), ("Bob", "鮑伯")]),
            Step::TranslateAll,
        ],
    );
    let request = TranslationRequest {
        glossary_text: Some("Alice = 艾莉絲".to_string()),
        context: None,
        glossary_entries: vec![
            GlossaryEntry {
                source: "Alice".to_string(),
                target: "艾莉絲".to_string(),
            },
            GlossaryEntry {
                source: "Carol".to_string(),
                target: "卡蘿".to_string(),
            },
            GlossaryEntry {
                source: "Dave".to_string(),
                target: "戴夫".to_string(),
            },
        ],
        ..progress_request()
    };

    let result = engine
        .translate_subtitle(make_subtitle(&["Hello Alice", "Goodbye Bob"]), &request)
        .await
        .unwrap();

    let observations = reporter.observations();
    assert_eq!(
        &observations[..3],
        &[
            Observed::Translation(TranslationProgress::TerminologyStarted),
            Observed::Provider("terminology"),
            Observed::Translation(TranslationProgress::TerminologyFinished { terms: 2 }),
        ],
        "the request must run between the terminology start and finish"
    );
    assert_eq!(
        result.outcome.effective_terminology.len(),
        4,
        "two generated terms plus three glossary entries, one of them an override"
    );
    assert_eq!(
        result.outcome.effective_terminology.get("Alice").unwrap(),
        "艾莉絲",
        "terms are counted before the glossary overrides the generated map"
    );
    assert_eq!(
        result.outcome.effective_terminology.get("Carol").unwrap(),
        "卡蘿"
    );
}

#[tokio::test]
async fn translation_progress_reports_each_batch_with_counts() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(
        &reporter,
        100,
        vec![
            no_terms(),
            Step::TranslateAll,
            Step::TranslateAll,
            Step::TranslateAll,
        ],
    );

    let result = engine
        .translate_subtitle(subtitle_with_cues(250), &progress_request())
        .await
        .unwrap();
    assert_eq!(result.outcome.batch_count, 3);

    assert_eq!(
        reporter.translations(),
        vec![
            TranslationProgress::TerminologyStarted,
            TranslationProgress::TerminologyFinished { terms: 0 },
            TranslationProgress::BatchStarted {
                pass: TranslationPass::Initial,
                batch: 1,
                batches: 3,
                cues: 100,
                attempt: 1,
            },
            TranslationProgress::BatchFinished {
                pass: TranslationPass::Initial,
                batch: 1,
                batches: 3,
                processed_cues: 100,
                total_cues: 250,
            },
            TranslationProgress::BatchStarted {
                pass: TranslationPass::Initial,
                batch: 2,
                batches: 3,
                cues: 100,
                attempt: 1,
            },
            TranslationProgress::BatchFinished {
                pass: TranslationPass::Initial,
                batch: 2,
                batches: 3,
                processed_cues: 200,
                total_cues: 250,
            },
            TranslationProgress::BatchStarted {
                pass: TranslationPass::Initial,
                batch: 3,
                batches: 3,
                cues: 50,
                attempt: 1,
            },
            TranslationProgress::BatchFinished {
                pass: TranslationPass::Initial,
                batch: 3,
                batches: 3,
                processed_cues: 250,
                total_cues: 250,
            },
        ]
    );
}

#[tokio::test]
async fn translation_progress_reports_missing_cues_and_fill_in() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(
        &reporter,
        100,
        vec![
            no_terms(),
            Step::TranslateOmitLast(4),
            Step::TranslateAll,
            Step::TranslateAll,
            Step::TranslateOmitLast(1),
        ],
    );

    let result = engine
        .translate_subtitle(subtitle_with_cues(250), &progress_request())
        .await
        .unwrap();
    assert_eq!(result.outcome.batch_count, 4, "three batches plus fill-in");

    let observations = reporter.observations();
    let events = reporter.translations();
    assert_eq!(
        events[3],
        TranslationProgress::BatchFinished {
            pass: TranslationPass::Initial,
            batch: 1,
            batches: 3,
            processed_cues: 96,
            total_cues: 250,
        },
        "four omitted cues do not count as processed"
    );
    let finish_index = observations
        .iter()
        .position(|observed| *observed == Observed::Translation(events[3]))
        .expect("the first BatchFinished is in the chronology");
    assert_eq!(
        observations[finish_index + 1],
        Observed::Message("📊 Translation Progress:\n   Processed cues: 96/250".to_string()),
        "the finish is immediately followed by its text line"
    );

    let fill_in: Vec<TranslationProgress> = events
        .iter()
        .copied()
        .filter(|event| {
            matches!(
                event,
                TranslationProgress::BatchStarted {
                    pass: TranslationPass::FillIn,
                    ..
                } | TranslationProgress::BatchFinished {
                    pass: TranslationPass::FillIn,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(
        fill_in,
        vec![
            TranslationProgress::BatchStarted {
                pass: TranslationPass::FillIn,
                batch: 1,
                batches: 1,
                cues: 4,
                attempt: 1,
            },
            TranslationProgress::BatchFinished {
                pass: TranslationPass::FillIn,
                batch: 1,
                batches: 1,
                processed_cues: 250,
                total_cues: 250,
            },
        ],
        "the fill-in finish counts the empty fallback as processed"
    );
}

#[tokio::test]
async fn translation_progress_clean_file_has_no_fill_in_or_stream() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(&reporter, 10, vec![no_terms(), Step::TranslateAll]);

    engine
        .translate_subtitle(make_subtitle(&["one", "two"]), &progress_request())
        .await
        .unwrap();

    let observations = reporter.observations();
    assert!(
        !observations.iter().any(|observed| matches!(
            observed,
            Observed::Translation(TranslationProgress::BatchStarted {
                pass: TranslationPass::FillIn,
                ..
            }) | Observed::Translation(TranslationProgress::BatchFinished {
                pass: TranslationPass::FillIn,
                ..
            })
        )),
        "nothing was missing, so no fill-in event may be reported"
    );
    assert!(
        !observations.iter().any(|observed| matches!(
            observed,
            Observed::Started | Observed::Advanced | Observed::Finished
        )),
        "translation progress must not open a unit-counted stream"
    );
}

#[tokio::test]
async fn translation_progress_announces_resend_with_attempt_two() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(
        &reporter,
        10,
        vec![no_terms(), Step::TranslateWithUnknownId, Step::TranslateAll],
    );

    let result = engine
        .translate_subtitle(make_subtitle(&["one", "two"]), &progress_request())
        .await
        .unwrap();
    assert_eq!(result.outcome.batch_count, 2);

    let events = reporter.translations();
    assert_eq!(
        events,
        vec![
            TranslationProgress::TerminologyStarted,
            TranslationProgress::TerminologyFinished { terms: 0 },
            TranslationProgress::BatchStarted {
                pass: TranslationPass::Initial,
                batch: 1,
                batches: 1,
                cues: 2,
                attempt: 1,
            },
            TranslationProgress::BatchStarted {
                pass: TranslationPass::Initial,
                batch: 1,
                batches: 1,
                cues: 2,
                attempt: 2,
            },
            TranslationProgress::BatchFinished {
                pass: TranslationPass::Initial,
                batch: 1,
                batches: 1,
                processed_cues: 2,
                total_cues: 2,
            },
        ],
        "the resend is a second start with attempt 2 and a single finish"
    );

    let observations = reporter.observations();
    assert_eq!(observations[5], Observed::Message(RETRY_NOTICE.to_string()));
    assert_eq!(observations[6], Observed::Translation(events[3]));
}

#[tokio::test]
async fn translation_progress_second_unknown_id_fails_without_finish() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(
        &reporter,
        10,
        vec![
            no_terms(),
            Step::TranslateWithUnknownId,
            Step::TranslateWithUnknownId,
        ],
    );

    let err = engine
        .translate_subtitle(make_subtitle(&["one", "two"]), &progress_request())
        .await
        .expect_err("the second unknown cue ID fails the file");
    assert!(
        err.to_string().contains("unknown cue ID after retry"),
        "unexpected: {err}"
    );

    assert_eq!(
        reporter.translations(),
        vec![
            TranslationProgress::TerminologyStarted,
            TranslationProgress::TerminologyFinished { terms: 0 },
            TranslationProgress::BatchStarted {
                pass: TranslationPass::Initial,
                batch: 1,
                batches: 1,
                cues: 2,
                attempt: 1,
            },
            TranslationProgress::BatchStarted {
                pass: TranslationPass::Initial,
                batch: 1,
                batches: 1,
                cues: 2,
                attempt: 2,
            },
        ],
        "a failed batch reports both starts and no finish"
    );
}

#[tokio::test]
async fn translation_progress_failed_terminology_emits_no_finish() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(
        &reporter,
        10,
        vec![Step::Fail("terminology request failed")],
    );

    let err = engine
        .translate_subtitle(make_subtitle(&["one"]), &progress_request())
        .await
        .expect_err("a failed terminology request fails the file");
    assert!(err.to_string().contains("terminology request failed"));
    assert_eq!(
        reporter.translations(),
        vec![TranslationProgress::TerminologyStarted],
        "the started event is reported, the finish is not"
    );
}

#[tokio::test]
async fn translation_progress_failed_batch_emits_no_finish() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(
        &reporter,
        10,
        vec![no_terms(), Step::Fail("translation request failed")],
    );

    let err = engine
        .translate_subtitle(make_subtitle(&["one", "two"]), &progress_request())
        .await
        .expect_err("a failed batch request fails the file");
    assert!(err.to_string().contains("translation request failed"));
    assert_eq!(
        reporter.translations(),
        vec![
            TranslationProgress::TerminologyStarted,
            TranslationProgress::TerminologyFinished { terms: 0 },
            TranslationProgress::BatchStarted {
                pass: TranslationPass::Initial,
                batch: 1,
                batches: 1,
                cues: 2,
                attempt: 1,
            },
        ],
        "a failed batch reports its start and no finish"
    );
}

#[tokio::test]
async fn text_progress_messages_are_unchanged_and_follow_batch_finishes() {
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(
        &reporter,
        3,
        vec![
            no_terms(),
            Step::TranslateAll,
            Step::TranslateAll,
            Step::TranslateAll,
        ],
    );

    engine
        .translate_subtitle(subtitle_with_cues(7), &progress_request())
        .await
        .unwrap();

    assert_eq!(
        reporter.messages(),
        vec![
            "📊 Translation Progress:\n   Processed cues: 3/7",
            "📊 Translation Progress:\n   Processed cues: 6/7",
            "📊 Translation Progress:\n   Processed cues: 7/7",
        ],
        "the text channel keeps the bytes it had before this change"
    );

    let observations = reporter.observations();
    for (index, observed) in observations.iter().enumerate() {
        if let Observed::Translation(TranslationProgress::BatchFinished {
            processed_cues,
            total_cues,
            ..
        }) = observed
        {
            match observations.get(index + 1) {
                Some(Observed::Message(message)) => assert!(
                    message.contains(&format!("Processed cues: {processed_cues}/{total_cues}")),
                    "BatchFinished {processed_cues}/{total_cues} not mirrored by {message:?}"
                ),
                other => panic!("BatchFinished must be followed by its message, got {other:?}"),
            }
        }
    }
}

#[tokio::test]
async fn empty_or_targetless_translation_reports_no_events() {
    // Empty subtitle: the engine returns before the terminology pass.
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(&reporter, 10, vec![]);
    let empty = Subtitle::new(
        SubtitleFormatType::Srt,
        SubtitleMetadata::new(SubtitleFormatType::Srt),
    );
    engine
        .translate_subtitle(empty, &progress_request())
        .await
        .unwrap();
    assert!(
        reporter.translations().is_empty(),
        "an empty subtitle reports no structured translation step"
    );

    // Empty target language: the configuration error returns before any
    // progress event and without calling the provider.
    let reporter = Arc::new(RecordingReporter::default());
    let engine = engine_with(&reporter, 10, vec![]);
    let mut request = progress_request();
    request.target_language = "   ".to_string();
    let err = engine
        .translate_subtitle(make_subtitle(&["one"]), &request)
        .await
        .expect_err("an empty target language is rejected");
    assert!(err.to_string().contains("target language"));
    assert!(
        reporter.observations().is_empty(),
        "no progress event may precede the target-language validation error"
    );
}

/// A reporter implementation written against the four variants that existed
/// before this change: it names each of them plus a wildcard and must keep
/// compiling when `ProgressEvent::Translation` is added.
fn classify_with_four_variants(event: &ProgressEvent<'_>) -> u8 {
    match event {
        ProgressEvent::Message(_) => 0,
        ProgressEvent::Started { .. } => 1,
        ProgressEvent::Advanced { .. } => 2,
        ProgressEvent::Finished { .. } => 3,
        _ => 4,
    }
}

#[test]
fn four_variant_match_receives_translation_in_wildcard() {
    let event = ProgressEvent::Translation(TranslationProgress::TerminologyStarted);
    assert_eq!(
        classify_with_four_variants(&event),
        4,
        "the new variant reaches a pre-existing wildcard arm"
    );
}
