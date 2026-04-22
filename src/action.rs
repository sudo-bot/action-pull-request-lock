//! Core decision logic for the lock action.

use anyhow::Result;

use crate::context::GithubContext;
use crate::github_client::GithubClient;
use crate::inputs::ActionInputs;
use crate::logger::Logger;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Locked,
}

pub async fn run(
    client: &dyn GithubClient,
    inputs: &ActionInputs,
    ctx: &GithubContext,
    log: &mut dyn Logger,
) -> Result<Outcome> {
    log.info(&format!(
        "Locking issue #{} with reason '{}'",
        inputs.number,
        inputs.lock_reason.as_str()
    ));

    client
        .lock_issue(
            &ctx.owner,
            &ctx.repo,
            inputs.number,
            inputs.lock_reason.as_str(),
        )
        .await?;

    log.info("Issue locked successfully.");
    Ok(Outcome::Locked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::GithubContext;
    use crate::inputs::{ActionInputs, LockReason};
    use crate::logger::CaptureLogger;
    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeClient {
        lock_calls: Mutex<Vec<(u64, String)>>,
        lock_err: Mutex<Option<String>>,
    }

    #[async_trait]
    impl GithubClient for FakeClient {
        async fn lock_issue(
            &self,
            _owner: &str,
            _repo: &str,
            issue_number: u64,
            lock_reason: &str,
        ) -> Result<()> {
            self.lock_calls
                .lock()
                .unwrap()
                .push((issue_number, lock_reason.to_string()));
            if let Some(e) = self.lock_err.lock().unwrap().clone() {
                return Err(anyhow::anyhow!(e));
            }
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

    fn inputs(reason: LockReason) -> ActionInputs {
        ActionInputs {
            github_token: "t".into(),
            number: 7,
            lock_reason: reason,
        }
    }

    #[tokio::test]
    async fn locks_with_resolved_reason() {
        let client = FakeClient::default();
        let mut log = CaptureLogger::new();
        let out = run(&client, &inputs(LockReason::Resolved), &ctx(), &mut log)
            .await
            .unwrap();
        assert_eq!(out, Outcome::Locked);

        let calls = client.lock_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], (7, "resolved".to_string()));
        assert!(log.contains("locked successfully"));
    }

    #[tokio::test]
    async fn locks_with_spam_reason() {
        let client = FakeClient::default();
        let mut log = CaptureLogger::new();
        let out = run(&client, &inputs(LockReason::Spam), &ctx(), &mut log)
            .await
            .unwrap();
        assert_eq!(out, Outcome::Locked);

        let calls = client.lock_calls.lock().unwrap();
        assert_eq!(calls[0], (7, "spam".to_string()));
    }

    #[tokio::test]
    async fn locks_with_off_topic_reason() {
        let client = FakeClient::default();
        let mut log = CaptureLogger::new();
        let out = run(&client, &inputs(LockReason::OffTopic), &ctx(), &mut log)
            .await
            .unwrap();
        assert_eq!(out, Outcome::Locked);

        let calls = client.lock_calls.lock().unwrap();
        assert_eq!(calls[0], (7, "off-topic".to_string()));
    }

    #[tokio::test]
    async fn locks_with_too_heated_reason() {
        let client = FakeClient::default();
        let mut log = CaptureLogger::new();
        let out = run(&client, &inputs(LockReason::TooHeated), &ctx(), &mut log)
            .await
            .unwrap();
        assert_eq!(out, Outcome::Locked);

        let calls = client.lock_calls.lock().unwrap();
        assert_eq!(calls[0], (7, "too heated".to_string()));
    }

    #[tokio::test]
    async fn lock_failure_propagates() {
        let client = FakeClient::default();
        *client.lock_err.lock().unwrap() = Some("API exploded".into());
        let mut log = CaptureLogger::new();
        let err = run(&client, &inputs(LockReason::Resolved), &ctx(), &mut log)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("API exploded"));
    }
}
