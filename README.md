# Lock a pull-request

This action locks a pull-request

## Example usage

```yml
  - name: lock pull request
    uses: sudo-bot/action-pull-request-lock@v1.0.4
    with:
        github-token: ${{ secrets.GITHUB_TOKEN }}
        number: ${{ github.event.pull_request.number }}
        lock_reason: resolved
```
