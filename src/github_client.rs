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
        let url = format!("/repos/{}/{}/issues/{}/lock", owner, repo, issue_number);
        let body = serde_json::json!({ "lock_reason": lock_reason });
        self.inner
            .put::<serde_json::Value, _, _>(url, Some(&body))
            .await
            .with_context(|| {
                format!(
                    "failed to lock issue #{} with reason '{}'",
                    issue_number, lock_reason
                )
            })?;
        Ok(())
    }
}
