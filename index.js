'use strict'

const core = require('@actions/core')
const { GitHub, context } = require('@actions/github')

const main = async () => {
  const token = core.getInput('github-token');
  const lock_reason = core.getInput('lock-reason');
  const number = core.getInput('number');

  const octokit = new GitHub(token);

  await octokit.issues.lock({
    lock_reason: lock_reason,
    ...context.repo,
    ...context.owner,
    issue_number: number,
  })
}

main().catch(err => core.setFailed(err.message))
