//! Read action inputs from environment variables.
//!
//! GitHub Actions exposes each `inputs.<name>` value as the environment
//! variable `INPUT_<NAME>` where `<NAME>` is the input name uppercased with
//! spaces (the runner does **not** translate hyphens; it simply uppercases).
//!
//! See the runner source: <https://github.com/actions/runner/blob/main/src/Runner.Worker/Container/ContainerInfo.cs>
//! and the `@actions/core` implementation: <https://github.com/actions/toolkit/blob/main/packages/core/src/core.ts>

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::env;

/// Match `@actions/core` getInput name normalisation: replace spaces with
/// underscores and uppercase, then prefix with `INPUT_`.
pub fn input_env_name(name: &str) -> String {
    format!("INPUT_{}", name.replace(' ', "_").to_uppercase())
}

/// Source of inputs. Production code uses [`EnvSource`]; tests use
/// [`MapSource`].
pub trait InputSource {
    fn get(&self, name: &str) -> Option<String>;
}

pub struct EnvSource;

impl InputSource for EnvSource {
    fn get(&self, name: &str) -> Option<String> {
        env::var(input_env_name(name)).ok()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MapSource {
    pub values: HashMap<String, String>,
}

impl MapSource {
    pub fn new<I, K, V>(it: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            values: it.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
        }
    }
}

impl InputSource for MapSource {
    fn get(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
}

/// The valid lock reasons accepted by the GitHub API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockReason {
    OffTopic,
    TooHeated,
    Resolved,
    Spam,
}

impl LockReason {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "off-topic" => Ok(Self::OffTopic),
            "too heated" => Ok(Self::TooHeated),
            "resolved" => Ok(Self::Resolved),
            "spam" => Ok(Self::Spam),
            other => Err(anyhow!(
                "invalid lock-reason '{}': expected one of off-topic, too heated, resolved, spam",
                other
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OffTopic => "off-topic",
            Self::TooHeated => "too heated",
            Self::Resolved => "resolved",
            Self::Spam => "spam",
        }
    }
}

/// Strongly typed inputs to the action. Mirrors `action.yml`.
#[derive(Debug, Clone)]
pub struct ActionInputs {
    pub github_token: String,
    pub number: u64,
    pub lock_reason: LockReason,
}

fn required<S: InputSource>(src: &S, name: &str) -> Result<String> {
    let value = src
        .get(name)
        .ok_or_else(|| anyhow!("Input required and not supplied: {}", name))?;
    if value.is_empty() {
        return Err(anyhow!("Input required and not supplied: {}", name));
    }
    Ok(value)
}

fn optional<S: InputSource>(src: &S, name: &str, default: &str) -> String {
    src.get(name)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

impl ActionInputs {
    pub fn from_source<S: InputSource>(src: &S) -> Result<Self> {
        let github_token = required(src, "github-token")?;
        let number_raw = required(src, "number")?;
        let number: u64 = number_raw
            .trim()
            .parse()
            .map_err(|e| anyhow!("Input 'number' must be a positive integer: {}", e))?;

        let lock_reason_raw = optional(src, "lock-reason", "resolved");
        let lock_reason = LockReason::parse(&lock_reason_raw)?;

        Ok(Self {
            github_token,
            number,
            lock_reason,
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::from_source(&EnvSource)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(pairs: &[(&str, &str)]) -> MapSource {
        MapSource::new(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
    }

    #[test]
    fn input_env_name_uppercases_and_keeps_hyphens() {
        assert_eq!(input_env_name("github-token"), "INPUT_GITHUB-TOKEN");
        assert_eq!(input_env_name("lock reason"), "INPUT_LOCK_REASON");
    }

    #[test]
    fn parses_full_input_set() {
        let s = src(&[
            ("github-token", "ghp_abc"),
            ("number", "42"),
            ("lock-reason", "spam"),
        ]);
        let inputs = ActionInputs::from_source(&s).unwrap();
        assert_eq!(inputs.github_token, "ghp_abc");
        assert_eq!(inputs.number, 42);
        assert_eq!(inputs.lock_reason, LockReason::Spam);
    }

    #[test]
    fn applies_defaults_when_optional_missing() {
        let s = src(&[("github-token", "ghp_abc"), ("number", "1")]);
        let inputs = ActionInputs::from_source(&s).unwrap();
        assert_eq!(inputs.lock_reason, LockReason::Resolved);
    }

    #[test]
    fn rejects_missing_token() {
        let s = src(&[("number", "1")]);
        let err = ActionInputs::from_source(&s).unwrap_err().to_string();
        assert!(err.contains("github-token"), "got: {}", err);
    }

    #[test]
    fn rejects_empty_required_input() {
        let s = src(&[("github-token", ""), ("number", "1")]);
        let err = ActionInputs::from_source(&s).unwrap_err().to_string();
        assert!(err.contains("github-token"));
    }

    #[test]
    fn rejects_non_numeric_number() {
        let s = src(&[("github-token", "x"), ("number", "abc")]);
        let err = ActionInputs::from_source(&s).unwrap_err().to_string();
        assert!(err.contains("number"));
    }

    #[test]
    fn rejects_unknown_lock_reason() {
        let s = src(&[
            ("github-token", "x"),
            ("number", "1"),
            ("lock-reason", "bored"),
        ]);
        let err = ActionInputs::from_source(&s).unwrap_err().to_string();
        assert!(err.contains("lock-reason"));
    }

    #[test]
    fn lock_reason_parses_all_variants() {
        assert_eq!(
            LockReason::parse("off-topic").unwrap(),
            LockReason::OffTopic
        );
        assert_eq!(
            LockReason::parse("too heated").unwrap(),
            LockReason::TooHeated
        );
        assert_eq!(LockReason::parse("resolved").unwrap(), LockReason::Resolved);
        assert_eq!(LockReason::parse("spam").unwrap(), LockReason::Spam);
        assert!(LockReason::parse("nope").is_err());
    }
}
