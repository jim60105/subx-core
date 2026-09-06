//! Structured progress streams and cooperative cancellation for the two
//! `MatchEngine` execution loops (`expose-core-orchestration-apis` D2).
//!
//! `execute_operations_audit` writes its resume journal under the XDG
//! config directory, so every test here pins `XDG_CONFIG_HOME` to a fresh
//! temp dir behind a mutex first (the pattern established by
//! `match_journal_tests.rs`; nextest process isolation makes the pin safe
//! per test anyway).
//!
//! What the delta spec's scenarios map to:
//! - "Full stream over a renamed batch" — `audit_stream_full_sequence`.
//! - "Empty batch is a real stream" — `audit_stream_empty_batch`.
//! - "Dry run emits nothing" — `audit_dry_run_emits_nothing` and
//!   `execute_dry_run_emits_nothing`.
//! - "Cancellation pads outcomes and closes short" —
//!   `audit_cancel_before_third_pads_outcomes_and_closes_short` and
//!   `audit_cancelled_before_first_closes_at_zero`.
//! - "No reporter attached changes no outcome" — `no_reporter_outcomes_unchanged`.
//! - The non-audit loop's stream (`item: None`; first-error abort closes
//!   with `done < total` and ignores `cancelled()`) —
//!   `execute_stream_full_sequence` and `execute_first_error_closes_short`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tempfile::TempDir;

use subx_core::core::matcher::discovery::{MediaFile, MediaFileType};
use subx_core::core::matcher::engine::{
    ConflictResolution, FileRelocationMode, MatchConfig, MatchEngine, MatchOperation,
};
use subx_core::core::report::{ProgressEvent, Reporter};
use subx_core::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};

/// Serialises the process-global `XDG_CONFIG_HOME` pin within this binary
/// (nextest gives each test its own process; a plain `cargo test` does not).
static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn make_config() -> MatchConfig {
    MatchConfig {
        confidence_threshold: 0.0,
        max_sample_length: 0,
        enable_content_analysis: false,
        backup_enabled: false,
        relocation_mode: FileRelocationMode::None,
        conflict_resolution: ConflictResolution::AutoRename,
        ai_model: "test-model".to_string(),
        max_subtitle_bytes: 52_428_800,
    }
}

struct PanicAI;

#[async_trait]
impl AIProvider for PanicAI {
    async fn analyze_content(&self, _req: AnalysisRequest) -> subx_core::Result<MatchResult> {
        panic!("AI provider must not be invoked during execute_operations tests")
    }

    async fn verify_match(&self, _req: VerificationRequest) -> subx_core::Result<ConfidenceScore> {
        panic!("AI provider must not be invoked during execute_operations tests")
    }
}

/// Records every progress event as a compact string, in order, and answers
/// `cancelled()` from the shared knobs.
struct RecordingReporter {
    events: Arc<Mutex<Vec<String>>>,
    /// When Some(n), the n-th `cancelled()` poll (1-based) is the first to
    /// return true.
    cancel_from_poll: Option<AtomicUsize>,
    cancel_all: AtomicBool,
}

impl RecordingReporter {
    /// Build a reporter plus the handle its events are visible through
    /// (the reporter moves into the engine; the handle stays with the test).
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                events: Arc::clone(&events),
                cancel_from_poll: None,
                cancel_all: AtomicBool::new(false),
            },
            events,
        )
    }

    /// Cancel from the `polls`-th `cancelled()` call onward.
    fn cancel_from_poll(mut self, polls: usize) -> Self {
        self.cancel_from_poll = Some(AtomicUsize::new(polls));
        self
    }

    fn cancel_immediately(self) -> Self {
        self.cancel_all.store(true, Ordering::SeqCst);
        self
    }
}

impl Reporter for RecordingReporter {
    fn progress(&self, event: &ProgressEvent<'_>) {
        let text = match event {
            ProgressEvent::Message(m) => format!("message:{m}"),
            ProgressEvent::Started { total } => format!("started:{total}"),
            ProgressEvent::Advanced { done, total, item } => {
                format!("advanced:{done}/{total}:{}", item.unwrap_or("-"))
            }
            ProgressEvent::Finished { done, total } => format!("finished:{done}/{total}"),
            // Future non_exhaustive variants would surface as "unknown";
            // the wildcard is the A1-mandated consumer shape.
            #[allow(unreachable_patterns)]
            _ => "unknown".to_string(),
        };
        self.events.lock().unwrap().push(text);
    }

    fn cancelled(&self) -> bool {
        if self.cancel_all.load(Ordering::SeqCst) {
            return true;
        }
        if let Some(counter) = &self.cancel_from_poll {
            let prev = counter.fetch_sub(1, Ordering::SeqCst);
            return prev <= 1;
        }
        false
    }
}

/// Pin the journal (and match cache, if any) into a private config home.
async fn config_home_guard() -> (TempDir, tokio::sync::MutexGuard<'static, ()>) {
    let guard = TEST_MUTEX.lock().await;
    let dir = TempDir::new().expect("create temp config dir");
    // SAFETY: TEST_MUTEX serialises every env mutation in this binary.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
    }
    (dir, guard)
}

fn engine() -> MatchEngine {
    MatchEngine::new(Box::new(PanicAI), make_config())
}

fn engine_with(reporter: RecordingReporter) -> MatchEngine {
    MatchEngine::new(Box::new(PanicAI), make_config()).with_reporter(Arc::new(reporter))
}

fn media_file(path: PathBuf, file_type: MediaFileType) -> MediaFile {
    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    MediaFile {
        id: name.clone(),
        path,
        file_type,
        size,
        name: name.clone(),
        extension,
        relative_path: name,
    }
}

/// A matched pair laid down on disk: subtitle beside the video, so
/// `relocation_mode: None` performs a plain in-place rename to
/// `<stem>.en.srt`.
fn pair(root: &Path, stem: &str) -> MatchOperation {
    let video = root.join(format!("{stem}.mkv"));
    let subtitle = root.join(format!("{stem}.srt"));
    fs::write(&video, b"video-bytes").unwrap();
    fs::write(
        &subtitle,
        format!("1\n00:00:01,000 --> 00:00:02,000\n{stem}\n"),
    )
    .unwrap();
    MatchOperation {
        video_file: media_file(video, MediaFileType::Video),
        subtitle_file: media_file(subtitle, MediaFileType::Subtitle),
        new_subtitle_name: format!("{stem}.en.srt"),
        confidence: 0.95,
        reasoning: vec!["same-directory".to_string()],
        relocation_mode: FileRelocationMode::None,
        relocation_target_path: None,
        requires_relocation: false,
    }
}

fn recorded(events: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    events.lock().unwrap().clone()
}

#[tokio::test]
async fn audit_stream_full_sequence() {
    let (_xdg, _guard) = config_home_guard().await;
    let work = TempDir::new().unwrap();
    let (reporter, events) = RecordingReporter::new();
    let engine = engine_with(reporter);
    let ops = vec![
        pair(work.path(), "a"),
        pair(work.path(), "b"),
        pair(work.path(), "c"),
    ];

    let outcomes = engine
        .execute_operations_audit(&ops, false)
        .await
        .expect("audit must not fail");

    // Stream contract: one Started, one Advanced per outcome, one Finished;
    // item names the subtitle file whose unit just completed.
    assert_eq!(
        recorded(&events),
        [
            "started:3",
            "advanced:1/3:a.srt",
            "advanced:2/3:b.srt",
            "advanced:3/3:c.srt",
            "finished:3/3",
        ]
    );
    assert!(outcomes.iter().all(|o| o.applied && o.error.is_none()));
    // The work really happened.
    assert!(work.path().join("a.en.srt").exists());
    assert!(!work.path().join("a.srt").exists());
}

#[tokio::test]
async fn audit_stream_empty_batch() {
    let (_xdg, _guard) = config_home_guard().await;
    let (reporter, events) = RecordingReporter::new();
    let engine = engine_with(reporter);

    let outcomes = engine
        .execute_operations_audit(&[], false)
        .await
        .expect("empty audit must not fail");

    assert!(outcomes.is_empty());
    assert_eq!(
        recorded(&events),
        ["started:0", "finished:0/0"],
        "an empty batch is a real, immediately-closed stream"
    );
}

#[tokio::test]
async fn audit_dry_run_emits_nothing() {
    let (_xdg, _guard) = config_home_guard().await;
    let work = TempDir::new().unwrap();
    let (reporter, events) = RecordingReporter::new();
    let engine = engine_with(reporter);
    let ops = vec![pair(work.path(), "a"), pair(work.path(), "b")];

    let outcomes = engine
        .execute_operations_audit(&ops, true)
        .await
        .expect("dry-run audit must not fail");

    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|o| !o.applied && o.error.is_none()));
    assert!(
        recorded(&events).is_empty(),
        "a dry run performs no work and opens no stream: {:?}",
        recorded(&events)
    );
    assert!(work.path().join("a.srt").exists(), "nothing renamed");
}

#[tokio::test]
async fn execute_dry_run_emits_nothing() {
    let (_xdg, _guard) = config_home_guard().await;
    let work = TempDir::new().unwrap();
    let (reporter, events) = RecordingReporter::new();
    let engine = engine_with(reporter);
    let ops = vec![pair(work.path(), "a")];

    engine
        .execute_operations(&ops, true)
        .await
        .expect("dry-run execute must not fail");

    assert!(
        recorded(&events).is_empty(),
        "dry run opens no stream: {:?}",
        recorded(&events)
    );
    assert!(work.path().join("a.srt").exists(), "nothing renamed");
}

#[tokio::test]
async fn audit_cancel_before_third_pads_outcomes_and_closes_short() {
    let (_xdg, _guard) = config_home_guard().await;
    let work = TempDir::new().unwrap();
    let (reporter, events) = RecordingReporter::new();
    // Third poll (1-based) is the first that returns true: operations 1 and
    // 2 run, operation 3 is never attempted.
    let engine = engine_with(reporter.cancel_from_poll(3));
    let ops = vec![
        pair(work.path(), "a"),
        pair(work.path(), "b"),
        pair(work.path(), "c"),
        pair(work.path(), "d"),
    ];

    let outcomes = engine
        .execute_operations_audit(&ops, false)
        .await
        .expect("cancellation never surfaces as Err");

    // Arity contract: one outcome per operation, always.
    assert_eq!(outcomes.len(), 4);
    assert!(outcomes[0].applied && outcomes[1].applied);
    for o in &outcomes[2..] {
        assert!(
            !o.applied && o.error.is_none(),
            "padded slots are {{applied: false, error: None}}"
        );
    }
    // Cancelled items were never attempted: files untouched on disk.
    assert!(work.path().join("a.en.srt").exists());
    assert!(work.path().join("b.en.srt").exists());
    assert!(work.path().join("c.srt").exists() && !work.path().join("c.en.srt").exists());
    assert!(work.path().join("d.srt").exists() && !work.path().join("d.en.srt").exists());

    // done captured BEFORE padding: never done == total on a cancel, and
    // the padded slots emit no Advanced.
    assert_eq!(
        recorded(&events),
        [
            "started:4",
            "advanced:1/4:a.srt",
            "advanced:2/4:b.srt",
            "finished:2/4",
        ]
    );
}

#[tokio::test]
async fn audit_cancelled_before_first_closes_at_zero() {
    let (_xdg, _guard) = config_home_guard().await;
    let work = TempDir::new().unwrap();
    let (reporter, events) = RecordingReporter::new();
    let engine = engine_with(reporter.cancel_immediately());
    let ops = vec![pair(work.path(), "a"), pair(work.path(), "b")];

    let outcomes = engine
        .execute_operations_audit(&ops, false)
        .await
        .expect("cancellation never surfaces as Err");

    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|o| !o.applied && o.error.is_none()));
    assert!(work.path().join("a.srt").exists(), "nothing attempted");
    assert_eq!(
        recorded(&events),
        ["started:2", "finished:0/2"],
        "cancel before any work: stream opens and closes at zero"
    );
}

#[tokio::test]
async fn no_reporter_outcomes_unchanged() {
    // The identity regression: with no reporter attached the audit loop's
    // outcomes are exactly what they were before emission/cancellation
    // existed — success shape, applied flags, errors, work on disk.
    let (_xdg, _guard) = config_home_guard().await;
    let work = TempDir::new().unwrap();
    let ops = vec![pair(work.path(), "a"), pair(work.path(), "b")];
    let outcomes = engine()
        .execute_operations_audit(&ops, false)
        .await
        .expect("noop engine must not fail");
    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|o| o.applied && o.error.is_none()));
    assert!(work.path().join("a.en.srt").exists());
    assert!(work.path().join("b.en.srt").exists());
}

#[tokio::test]
async fn execute_stream_full_sequence() {
    let (_xdg, _guard) = config_home_guard().await;
    let work = TempDir::new().unwrap();
    let (reporter, events) = RecordingReporter::new();
    let engine = engine_with(reporter);
    let ops = vec![pair(work.path(), "a"), pair(work.path(), "b")];

    engine
        .execute_operations(&ops, false)
        .await
        .expect("execute must succeed");

    assert_eq!(
        recorded(&events),
        [
            "started:2",
            "advanced:1/2:-",
            "advanced:2/2:-",
            "finished:2/2"
        ],
        "the non-audit loop carries no per-unit item"
    );
    assert!(work.path().join("a.en.srt").exists());
}

#[tokio::test]
async fn execute_first_error_closes_short() {
    // The non-audit loop aborts on the first failure and never polls
    // cancellation; its stream closes with done < total (the stream
    // contract blesses an early stop; the Err carries the failure).
    let (_xdg, _guard) = config_home_guard().await;
    let work = TempDir::new().unwrap();
    // cancel_immediately proves Decision 6 from the other side: the
    // non-audit loop IGNORES cancelled().
    let (reporter, events) = RecordingReporter::new();
    let engine = engine_with(reporter.cancel_immediately());

    // First op renames a real file; the SECOND op's source is deleted
    // after construction so its rename fails mid-batch.
    let first = pair(work.path(), "a");
    let mut second = pair(work.path(), "b");
    fs::remove_file(work.path().join("b.srt")).unwrap();
    second.subtitle_file.path = work.path().join("b.srt");

    let err = engine
        .execute_operations(&[first, second], false)
        .await
        .expect_err("renaming a deleted source must fail the batch");
    assert!(matches!(
        err,
        subx_core::error::SubXError::FileOperationFailed(_)
    ));

    assert_eq!(
        recorded(&events),
        ["started:2", "advanced:1/2:-", "finished:1/2"],
        "one completed unit, early close at done < total, cancellation ignored"
    );
    // The completed unit's work persists.
    assert!(work.path().join("a.en.srt").exists());
}
