# Lock a pull-request

This action locks a pull-request

## Example usage

```yml
  - name: lock pull request
    uses: sudo-bot/action-pull-request-lock@v1
    with:
        github-token: ${{ secrets.WDES_BOT_TOKEN }}
        number: ${{ secrets.GITHUB_TOKEN }}
        lock_reason: resolved
```
