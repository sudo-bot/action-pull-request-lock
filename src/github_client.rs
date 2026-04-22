//! Trait abstraction over the GitHub REST API surface we need.
//!
//! Defining a trait keeps `octocrab` (the real HTTP client) at arm's length
//! so the action's decision logic can be unit-tested with a fake.

use anyhow::{Context as _, Result};
use async_trait::async_trait;

#[async_trait]
pub trait GithubClient: Send + Sync {
    /// Lock an issue/pull-request conversation.
    /// `PUT /repos/{owner}/{repo}/issues/{issue_number}/lock`
    async fn lock_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        lock_reason: &str,
    ) -> Result<()>;
}

/// Real implementation backed by `octocrab`.
pub struct OctocrabClient {
    inner: octocrab::Octocrab,
}

impl OctocrabClient {
    pub fn new(token: String, base_url: &str) -> Result<Self> {
        let mut builder = octocrab::Octocrab::builder().personal_token(token);
        // Allow GitHub Enterprise Server by honouring GITHUB_API_URL.
        if base_url != "https://api.github.com" {
            builder = builder
                .base_uri(base_url)
                .context("invalid GITHUB_API_URL")?;
        }
        let inner = builder.build().context("failed to build octocrab client")?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl GithubClient for OctocrabClient {
    async fn lock_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        lock_reason: &str,
    ) -> Result<()> {
        let route = format!("/repos/{}/{}/issues/{}/lock", owner, repo, issue_number);
        let body = serde_json::json!({ "lock_reason": lock_reason });
        let response = self.inner._put(route, Some(&body)).await.with_context(|| {
            format!(
                "failed to lock issue #{} with reason '{}'",
                issue_number, lock_reason
            )
        })?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!(
                "failed to lock issue #{} with reason '{}': unexpected status {}",
                issue_number,
                lock_reason,
                status
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn lock_issue_handles_204_no_content() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/octo/widget/issues/42/lock"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        let client = OctocrabClient::new("fake-token".into(), &server.uri()).unwrap();
        client
            .lock_issue("octo", "widget", 42, "resolved")
            .await
            .expect("204 No Content should not cause a JSON parse error");
    }

    #[tokio::test]
    async fn lock_issue_fails_on_non_success_status() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/octo/widget/issues/42/lock"))
            .respond_with(ResponseTemplate::new(403))
            .expect(1)
            .mount(&server)
            .await;

        let client = OctocrabClient::new("fake-token".into(), &server.uri()).unwrap();
        let err = client
            .lock_issue("octo", "widget", 42, "resolved")
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("failed to lock issue #42"),
            "unexpected error: {msg}"
        );
    }
}
