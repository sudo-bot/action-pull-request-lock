# Lock a pull-request

This action locks a pull-request conversation.

## Inputs

| Input          | Required | Default    | Description                                              |
| -------------- | :------: | ---------- | -------------------------------------------------------- |
| `github-token` |   yes    | —          | GitHub token with `issues: write` permission.            |
| `number`       |   yes    | —          | Pull-request (issue) number to lock.                     |
| `lock-reason`  |    no    | `resolved` | One of: `off-topic`, `too heated`, `resolved`, `spam`.   |

## Permissions

```yaml
permissions:
    issues: write          # lock the conversation
    pull-requests: write   # (optional) if the token also needs PR scope
```

## Example usage

```yml
name: lock pull-request
on:
    pull_request:
        types:
            - closed
jobs:
    lock:
        runs-on: ubuntu-latest
        permissions:
            issues: write
            pull-requests: write
        steps:
            - name: lock pull request
              uses: sudo-bot/action-pull-request-lock@v2
              with:
                  github-token: ${{ secrets.GITHUB_TOKEN }}
                  number: ${{ github.event.pull_request.number }}
                  lock-reason: resolved
```

## How it works

1. The action reads the inputs from the workflow.
2. It calls the GitHub REST API to lock the issue/pull-request with the
   specified reason.
3. If the API call fails, the action exits with a non-zero code and prints
   an error message.

## Gitea (self-hosted) support

The action also runs on [Gitea Actions](https://docs.gitea.com/usage/actions/overview).
Detection is automatic: when the runner sets `GITEA_ACTIONS=true` (or
`GITHUB_API_URL` ends in `/api/v1`), the action talks to Gitea's REST
API instead of GitHub's. Gitea's lock endpoint
(`PUT /repos/{o}/{r}/issues/{n}/lock` with `{ "lock_reason": "..." }`)
is wire-compatible with GitHub's, so **the workflow above works
unchanged** — no separate example needed. The same `lock-reason` values
apply.

The only practical caveat is image distribution: your Gitea runner
must be able to pull `ghcr.io/sudo-bot/action-pull-request-lock:latest`.
If the runner is restricted to a private registry, mirror the image
there.
