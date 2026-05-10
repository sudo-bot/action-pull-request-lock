//! Gitea-specific implementation of [`GithubClient`].
//!
//! Gitea's lock endpoint is wire-compatible with GitHub's:
//!
//! - `PUT /repos/{owner}/{repo}/issues/{index}/lock` on both forges
//! - Body shape: `{ "lock_reason": "..." }` on both forges
//! - 204 No Content on success on both forges
//!
//! So this client's `lock_issue` implementation is currently identical to
//! [`OctocrabClient`]'s. The separate type exists for two reasons:
//!
//! 1. Structural parity with the `action-pull-request-merge` sister
//!    project, where Gitea genuinely diverges (POST instead of PUT, label
//!    removal by id) and a dedicated `GiteaClient` is required.
//! 2. A place for future Gitea-specific divergences to land if they
//!    appear (e.g. if Gitea adds custom lock reasons or changes the body
//!    shape) without disturbing the GitHub path.
//!
//! [`OctocrabClient`]: crate::github_client::OctocrabClient

use anyhow::{Context as _, Result};
use async_trait::async_trait;

use crate::github_client::GithubClient;

pub struct GiteaClient {
    inner: octocrab::Octocrab,
}

impl GiteaClient {
    pub fn new(token: String, base_url: &str) -> Result<Self> {
        let inner = octocrab::Octocrab::builder()
            .personal_token(token)
            .base_uri(base_url)
            .context("invalid GITHUB_API_URL for Gitea")?
            .build()
            .context("failed to build octocrab client for Gitea")?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl GithubClient for GiteaClient {
    async fn lock_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        lock_reason: &str,
    ) -> Result<()> {
        let route = format!("/repos/{}/{}/issues/{}/lock", owner, repo, issue_number);
        let response = self
            .inner
            ._put(
                route.clone(),
                Some(&serde_json::json!({ "lock_reason": lock_reason })),
            )
            .await
            .with_context(|| {
                format!(
                    "failed to send lock for issue #{} (PUT {}) with reason '{}'",
                    issue_number, route, lock_reason
                )
            })?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!(
                "failed to lock issue #{} (PUT {}) with reason '{}': unexpected status {}",
                issue_number,
                route,
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
    use serde_json::json;
    use wiremock::matchers::{body_json, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn gitea_client_new_rejects_invalid_base_url() {
        let result = GiteaClient::new("token".into(), "::not a uri::");
        let err = result.err().expect("garbage URL must not build a client");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("invalid GITHUB_API_URL")
                || msg.to_lowercase().contains("uri")
                || msg.to_lowercase().contains("url"),
            "unexpected error message: {}",
            msg
        );
    }

    #[tokio::test]
    async fn gitea_client_new_accepts_typical_gitea_base_url() {
        let result = GiteaClient::new("token".into(), "https://gitea.example.com/api/v1");
        assert!(result.is_ok(), "expected build to succeed for valid URL");
    }

    #[tokio::test]
    async fn gitea_lock_issue_uses_put_with_lock_reason_body_and_auth_header() {
        // Pin the wire shape: PUT (not POST), the right path, the
        // lock_reason body, and an Authorization header. A regression
        // that flipped to POST or omitted auth would compile and pass
        // every other test, but fail here.
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/octo/widget/issues/42/lock"))
            .and(header_exists("authorization"))
            .and(body_json(json!({ "lock_reason": "resolved" })))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        let client = GiteaClient::new("fake-token".into(), &server.uri()).unwrap();
        client
            .lock_issue("octo", "widget", 42, "resolved")
            .await
            .expect("Gitea returns 204 No Content on success");
    }

    #[tokio::test]
    async fn gitea_lock_issue_surfaces_4xx_with_status_in_message() {
        // A non-2xx response from Gitea (e.g. 403 if the token lacks
        // permission) must propagate as a failure that names the issue
        // and the status code, not a silently-handled success.
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/octo/widget/issues/42/lock"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = GiteaClient::new("fake-token".into(), &server.uri()).unwrap();
        let err = client
            .lock_issue("octo", "widget", 42, "resolved")
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("#42"), "msg should name the issue: {}", msg);
        assert!(msg.contains("403"), "msg should name the status: {}", msg);
        // The URL path must be present so a 405 / 404 from a misconfigured
        // Gitea instance is diagnosable without re-running.
        assert!(
            msg.contains("/repos/octo/widget/issues/42/lock"),
            "expected URL path in error for diagnostics: {}",
            msg
        );
    }
}
