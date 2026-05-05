# action-pull-request-lock

GitHub / Gitea Action that locks a pull-request conversation when an
event triggers the workflow. Written in Rust, distributed as a Docker
container action — no Node.js runtime required on the runner.

Inputs: `github-token`, `number`, `lock-reason`. See `action.yml` and
`README.md`.

## Layout

```
src/
  main.rs            Tiny entrypoint. Picks the client backend and hands off.
  lib.rs             Re-exports + pick_backend(&ctx) -> Backend.
  action.rs          Decision logic: log, call lock_issue, log success.
                     Trivial because the action does one thing.
  context.rs         Reads GITHUB_* env vars. Detects Gitea via
                     GITEA_ACTIONS=true OR a `/api/v1` URL suffix.
  inputs.rs          Reads INPUT_<NAME> env vars (matches @actions/core
                     name normalisation). LockReason enum.
  github_client.rs   GithubClient trait + OctocrabClient (real GitHub
                     impl). Single trait method: lock_issue.
  gitea_client.rs    GiteaClient. Currently identical to OctocrabClient
                     because Gitea's lock endpoint is wire-compatible.
                     Kept as a separate type for sister-project parity.
  logger.rs          Logger trait, WriteLogger<W: Write>, StdoutLogger
                     (= WriteLogger<io::Stdout>), CaptureLogger (test).
tests/
  integration.rs     action::run driven by a fake client.
docker/
  Dockerfile         Multi-stage: rust:1-alpine build, alpine:3.23 runtime.
.github/workflows/   build / lock / release.
```

## How it's wired

`action::run` knows nothing about HTTP. It calls a single trait method
on a `&dyn GithubClient`:

```
lock_issue   PUT /repos/{o}/{r}/issues/{n}/lock with { lock_reason }
```

Two implementations: `OctocrabClient` (GitHub) and `GiteaClient`. Both
use `octocrab::Octocrab` purely as an authenticated HTTP client; the
typed GitHub helpers from octocrab are *not* used. Selection happens
once at startup via `pick_backend(&ctx)`.

### Gitea compatibility

Gitea's lock endpoint is wire-compatible with GitHub's: same path,
method (`PUT`), body shape (`{ "lock_reason": "..." }`), and 204 No
Content on success. So `GiteaClient::lock_issue` is currently identical
to `OctocrabClient::lock_issue`. The dedicated type exists for
structural parity with the
[action-pull-request-merge](https://github.com/sudo-bot/action-pull-request-merge)
sister project (where Gitea genuinely diverges) and gives any future
Gitea-specific lock divergences a place to land.

## Build / test / lint

```sh
cargo build --release          # what the Dockerfile runs
cargo test                     # 36 tests across unit + integration
cargo fmt --check              # style
cargo clippy --all-targets -- -D warnings   # lints
make docker-build              # local image build (linux/amd64 by default)
```

Pre-push gate: `cargo fmt --check && cargo clippy --all-targets -- -D
warnings && cargo test`.

## Testing model

Two layers:

1. **Unit tests in each module.** Pure-function level: env parsing,
   `pick_backend`, `escape_data`, log-byte verification via
   `WriteLogger<Vec<u8>>`, plus `wiremock` tests in
   `src/{github,gitea}_client.rs::tests` that pin the exact HTTP method,
   path, body and `Authorization` header each client sends.
2. **`tests/integration.rs`.** Drives `action::run` against a fake
   `GithubClient` to verify the decision flow end-to-end. No HTTP.

When changing how a request is built, add or extend the wire test next
to the affected client.

## Conventions / gotchas

- **Env-touching tests must use `with_env` in `context.rs`.** It holds a
  process-wide `Mutex` so parallel tests can't observe a half-mutated
  environment.
- **Trait-first plumbing.** Don't add HTTP work directly into
  `action.rs`. Add a method to the `GithubClient` trait, implement it
  on both `OctocrabClient` and `GiteaClient`, and add a wire test on
  each side.
- **Errors are propagated, not logged-and-swallowed.** A 4xx/5xx from
  the lock endpoint fails the action with the status code and issue
  number in the message.
- **Outputs go through the `Logger` trait.** Don't `println!` from
  library code — write to the logger so tests can capture it.

## Distribution

- Docker image: `ghcr.io/sudo-bot/action-pull-request-lock:latest`.
- Marketplace tag: `@v2` (moving). `Cargo.toml` stays on `2.0.0`.
  `make update-tags` re-points `v2` at `main` and force-pushes.

## Sister project: keeping in sync with action-pull-request-merge

This repo shares ~98% of its scaffolding with
[`sudo-bot/action-pull-request-merge`](https://github.com/sudo-bot/action-pull-request-merge):
the `GithubClient` trait pattern, env-based context, workflow-command
logger, fake-client integration tests, `WriteLogger<W>`, the
`is_gitea` detection rule, `pick_backend`, the `with_env` mutex, the
Dockerfile shape. **Their git histories are unified** (a single
`--allow-unrelated-histories` merge sits in this repo's log) so
shared scaffolding work can flow between repositories without manual
re-implementation.

### Setup

Add the sister repo as a remote:

```sh
git remote add merge-action git@github.com:sudo-bot/action-pull-request-merge.git
git fetch merge-action
```

### Files that should stay identical

Pure scaffolding with no domain content. If they drift, that drift is
almost always a bug:

- `src/logger.rs`

### Files that should track each other but allow domain divergence

The structure (function signatures, test patterns, error messages)
should match; the data inside differs:

- `src/context.rs` — same `is_gitea` detection, same `with_env` helper
  and `ENV_LOCK` mutex. Lock-action additionally omits the `actor`
  field that merge-action needs for its `allowed-usernames-regex`
  gate.
- `src/lib.rs` — both expose `Backend`, `pick_backend`, the same
  re-exports skeleton. Lock has fewer `inputs::*` re-exports.
- `src/main.rs` — identical apart from the package name in `use`.
- `Cargo.toml` — same dep set apart from `regex` (lock doesn't
  pattern-match anything) and package metadata. License is MPL-2.0 in
  both.
- `docker/Dockerfile`, `Makefile` — identical apart from the image
  name.

### Files that are intentionally divergent

Domain-specific. Don't try to keep these aligned beyond high-level
patterns:

- `src/action.rs`, `src/inputs.rs`,
  `src/github_client.rs`, `src/gitea_client.rs`,
  `tests/integration.rs`.
- `action.yml`, `README.md`, `CHANGELOG.md`.

### Workflow

When you write a scaffolding change here:

1. Land it in this repo.
2. `cd ../action-pull-request-merge && git fetch <this-remote>`.
3. Cherry-pick the commit (`git cherry-pick <sha>`), or apply by hand
   if surrounding code has drifted.
4. Run that repo's `cargo test && cargo clippy && cargo fmt --check`.

When you find drift on a should-be-identical file, reconcile it
deliberately rather than letting each side mutate.

A useful diff:

```sh
diff -rq ../action-pull-request-merge/src ./src \
  | grep -v 'gitea_client\|github_client\|action\.rs\|inputs\.rs'
```

— anything else flagged is a candidate for sync.
