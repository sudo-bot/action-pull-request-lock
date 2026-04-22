//! End-to-end integration tests. They drive `action::run` through a fake
//! GitHub client while exercising realistic input/environment wiring.

use action_pull_request_lock::action::{run, Outcome};
use action_pull_request_lock::context::GithubContext;
use action_pull_request_lock::github_client::GithubClient;
use action_pull_request_lock::inputs::{ActionInputs, LockReason, MapSource};
use action_pull_request_lock::logger::CaptureLogger;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Mutex;

#[derive(Default)]
struct FakeClient {
    lock_calls: Mutex<Vec<(u64, String)>>,
}

#[async_trait]
impl GithubClient for FakeClient {
    async fn lock_issue(
        &self,
        _o: &str,
        _r: &str,
        issue_number: u64,
        lock_reason: &str,
    ) -> Result<()> {
        self.lock_calls
            .lock()
            .unwrap()
            .push((issue_number, lock_reason.to_string()));
        Ok(())
    }
}

fn ctx() -> GithubContext {
    GithubContext {
        owner: "octo".into(),
        repo: "widget".into(),
        api_base_url: "https://api.github.com".into(),
    }
}

#[tokio::test]
async fn end_to_end_locks_with_default_reason() {
    let src = MapSource::new([("github-token", "ghp_test"), ("number", "42")]);
    let inputs = ActionInputs::from_source(&src).unwrap();
    assert_eq!(inputs.number, 42);
    assert_eq!(inputs.lock_reason, LockReason::Resolved);

    let client = FakeClient::default();
    let mut log = CaptureLogger::new();
    let out = run(&client, &inputs, &ctx(), &mut log).await.unwrap();
    assert_eq!(out, Outcome::Locked);

    let calls = client.lock_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (42, "resolved".to_string()));
}

#[tokio::test]
async fn end_to_end_locks_with_custom_reason() {
    let src = MapSource::new([
        ("github-token", "ghp_test"),
        ("number", "7"),
        ("lock-reason", "spam"),
    ]);
    let inputs = ActionInputs::from_source(&src).unwrap();
    assert_eq!(inputs.lock_reason, LockReason::Spam);

    let client = FakeClient::default();
    let mut log = CaptureLogger::new();
    let out = run(&client, &inputs, &ctx(), &mut log).await.unwrap();
    assert_eq!(out, Outcome::Locked);

    let calls = client.lock_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (7, "spam".to_string()));
}
