# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v2.1.0] - 2026-05-05

### Added
- Gitea self-hosted support. The action auto-detects Gitea Actions
  (via the `GITEA_ACTIONS=true` env var the runner sets, or a `/api/v1`
  suffix on `GITHUB_API_URL`) and routes the lock call through a new
  `GiteaClient`. Gitea's lock endpoint is wire-compatible with GitHub's
  (`PUT /repos/{o}/{r}/issues/{n}/lock` with `{ "lock_reason": "..." }`),
  so existing workflows run unchanged on Gitea — the dedicated client
  exists for structural parity with the
  [action-pull-request-merge](https://github.com/sudo-bot/action-pull-request-merge)
  sister project and gives any future Gitea-specific divergences a
  place to land.
- README section noting Gitea support and the ghcr.io reachability
  caveat for self-hosted runners.

### Changed
- `StdoutLogger` is now a type alias for a generic
  `WriteLogger<W: Write>`, letting tests verify the exact bytes
  emitted (workflow command prefixes and escape pass) instead of just
  trusting `println!`.

### Internal
- Histories merged with the action-pull-request-merge sister project so
  shared scaffolding can flow between repositories without manual
  cherry-pick.
- `pick_backend(&ctx) -> Backend` extracted in `lib.rs` with two unit
  tests, so the GitHub/Gitea selection rule has direct coverage instead
  of being a buried `if`-`else` in `main.rs`.
- `with_env` test helper now serialises through a process-wide `Mutex`
  so parallel env-touching tests don't observe a half-mutated
  environment.
- New wire tests pin `GiteaClient`'s HTTP shape (`PUT`, the right path,
  the `lock_reason` body, `Authorization` header presence, 4xx
  propagation).
- Test count grew from 13 to 36.

## [v2.0.0] - 2026-04-22

### Changed
- **Action rewritten in Rust.** The action is now a single Rust binary
  shipped as a Docker container action and no longer requires a Node.js
  runtime on the runner. API calls go through
  [`octocrab`](https://crates.io/crates/octocrab). The user-facing
  inputs surface (`github-token`, `number`, `lock-reason`) is unchanged.
- Distribution moved to GitHub Container Registry
  (`ghcr.io/sudo-bot/action-pull-request-lock`), published on the
  `latest` tag in addition to versioned tags.

### Fixed
- A 204 No Content response from the GitHub lock endpoint no longer
  causes a JSON deserialisation error. The client uses the low-level
  `_put` helper and checks the status manually.

## [v1.2.0] - 2022-07-10

- Mark `lock-reason` as not required (it has a default value).
- Bump version number.
- Use Node 16 instead of Node 12.
- Upgrade dependencies.

## [v1.1.1] - 2020-06-03

- Documentation updates.

## [v1.1.0] - 2022-07-10

- Maintenance release.

## [v1.0.5] - 2019-12-08

- Bug fixes.

## [v1.0.4] - 2019-12-08

- Bug fixes.

## [v1.0.3] - 2019-12-08

- First working version.

## [v1.0.2] - 2019-12-08

- Some fixes.

## [v1.0.1] - 2019-12-08

- Some fixes.

## [v1.0.0] - 2019-12-08

- First stable version.

[v2.1.0]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.2.0...v2
[v1.2.0]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.1.1...v1.2.0
[v1.1.1]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.1.0...v1.1.1
[v1.1.0]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.0.5...v1.1.0
[v1.0.5]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.0.4...v1.0.5
[v1.0.4]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.0.3...v1.0.4
[v1.0.3]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.0.2...v1.0.3
[v1.0.2]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.0.1...v1.0.2
[v1.0.1]: https://github.com/sudo-bot/action-pull-request-lock/compare/v1.0.0...v1.0.1
[v1.0.0]: https://github.com/sudo-bot/action-pull-request-lock/releases/tag/v1.0.0
