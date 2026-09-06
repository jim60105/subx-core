//! Behavioural proof for `ComponentFactory::create_match_engine_with`
//! (`expose-core-orchestration-apis` D2, gap 3).
//!
//! The unit tests in `factory.rs` prove `match_config()`'s fields and that
//! `create_match_engine` delegates through the same two methods, but
//! `relocation_mode` only becomes observable once a matched
//! `MatchOperation` is produced — which needs a wired AI client. Here the
//! mock OpenAI server from `test_support` closes that gap: a factory whose
//! engine is built with `relocation_mode: Copy` MUST emit relocating
//! operations, and the same factory on the default `match_config()` MUST
//! not.

use subx_core::config::TestConfigBuilder;
use subx_core::core::ComponentFactory;
use subx_core::core::matcher::engine::{FileRelocationMode, MatchOperation};
use subx_core::test_support::mock_openai::MockOpenAITestHelper;
use tempfile::TempDir;

/// Pair a root-level video with a subdirectory subtitle through a factory
/// engine built via `create_match_engine_with(factory.match_config())`
/// after applying `configure`. The `TempDir` is returned so the caller
/// keeps the files alive while inspecting operation paths.
async fn matched_ops(
    configure: impl FnOnce(&mut subx_core::core::matcher::MatchConfig),
) -> (TempDir, Vec<MatchOperation>) {
    let mock = MockOpenAITestHelper::new().await;
    mock.mock_chat_completion_echoing_request_ids(1, 1, 0.95)
        .await;

    let root = TempDir::new().unwrap();
    let video = root.path().join("movie.mkv");
    let sub_dir = root.path().join("subs");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(&video, b"video-bytes").unwrap();
    std::fs::write(
        sub_dir.join("movie.srt"),
        b"1\n00:00:01,000 --> 00:00:02,000\nhi\n",
    )
    .unwrap();

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock.base_url())
        .build_service();
    let factory = ComponentFactory::new(&config_service).expect("factory");

    let mut match_config = factory.match_config();
    configure(&mut match_config);
    let engine = factory
        .create_match_engine_with(match_config)
        .expect("engine with mock AI");

    let operations = engine
        .match_file_list(&[video, sub_dir.join("movie.srt")])
        .await
        .expect("mock AI must produce a match");

    (root, operations)
}

#[tokio::test]
async fn create_match_engine_with_copy_mode_reaches_operations() {
    let (_root, operations) = matched_ops(|c| c.relocation_mode = FileRelocationMode::Copy).await;

    assert_eq!(operations.len(), 1, "the echo mock pairs the one video");
    let op = &operations[0];
    // The caller's override — not the factory default — is what the engine
    // used: the subdirectory subtitle is relocated next to the video.
    assert_eq!(op.relocation_mode, FileRelocationMode::Copy);
    assert!(op.requires_relocation);
    assert_eq!(
        op.relocation_target_path.as_deref(),
        Some(_root.path().join("movie.srt").as_path()),
        "target must be the new subtitle name inside the video's directory"
    );
}

#[tokio::test]
async fn default_match_config_does_not_relocate() {
    // Same scene, factory defaults: requires_relocation stays false, which
    // is the regression that `create_match_engine()`'s behaviour did not
    // change when it began delegating.
    let (_root, operations) = matched_ops(|_| {}).await;

    assert_eq!(operations.len(), 1);
    let op = &operations[0];
    assert_eq!(op.relocation_mode, FileRelocationMode::None);
    assert!(!op.requires_relocation);
    assert_eq!(op.relocation_target_path, None);
}
