const Handlebars = require('handlebars');
const fs = require('fs');
const path = require('path');
const { Octokit, App } = require('octokit');

async function addToContextAndValidate(context) {
  if (!context.env.WORKFLOW_CONCLUSION) {
    throw new Error('Please ensure the env var WORKFLOW_CONCLUSION is set');
  }
  if (!context.env.GITHUB_TOKEN) {
    throw new Error('Please ensure the env var GITHUB_TOKEN is set');
  }
  if (!context.env.GITHUB_RUN_ID) {
    throw new Error('Please ensure the env var GITHUB_RUN_ID is set');
  }
  if (!context.env.GITHUB_REPOSITORY) {
    throw new Error('Please ensure the env var GITHUB_REPOSITORY is set');
  }
}

async function getMessageBody(context) {
  const source = fs
    .readFileSync(
      context.env.WORKFLOW_CONCLUSION === 'success'
        ? path.resolve(__dirname, 'templates', 'success')
        : path.resolve(__dirname, 'templates', 'failure'),
    )
    .toString();
  const template = Handlebars.compile(source);

  // get job details from GitHub API
  const octokit = new Octokit({ auth: context.env.GITHUB_TOKEN });
  const [owner, repo] = context.env.GITHUB_REPOSITORY.split('/');
  const {
    data: { jobs },
  } = await octokit.rest.actions.listJobsForWorkflowRun({
    run_id: context.env.GITHUB_RUN_ID,
    owner,
    repo,
  });

  // uncomment this to see what is available for each job
  if(process.env.SHOW_DEBUG) {
    console.dir(jobs, { depth: null });
  }

  /*

   a sample of the response is:

    {
      total_count: 10,
      jobs: [
        {
          id: 5182940024,
          run_id: 1840752095,
          run_url: 'https://api.github.com/repos/nymtech/nym/actions/runs/1840752095',
          run_attempt: 1,
          node_id: 'CR_kwDODdjOis8AAAABNO1jeA',
          head_sha: 'aa00eb70d57751bfa556bd3602df87c7473367fc',
          url: 'https://api.github.com/repos/nymtech/nym/actions/jobs/5182940024',
          html_url: 'https://github.com/nymtech/nym/runs/5182940024?check_suite_focus=true',
          status: 'completed',
          conclusion: 'success',
          started_at: '2022-02-14T11:28:34Z',
          completed_at: '2022-02-14T11:28:38Z',
          name: 'matrix_prep',
          steps: [
            {
              name: 'Set up job',
              status: 'completed',
              conclusion: 'success',
              number: 1,
              started_at: '2022-02-14T13:28:34.000+02:00',
              completed_at: '2022-02-14T13:28:36.000+02:00'
            },
            {
              name: 'Run actions/checkout@v2',
              status: 'completed',
              conclusion: 'success',
              number: 2,
              started_at: '2022-02-14T13:28:36.000+02:00',
              completed_at: '2022-02-14T13:28:37.000+02:00'
            },
            ...
          ],
          check_run_url: 'https://api.github.com/repos/nymtech/nym/check-runs/5182940024',
          labels: [ 'ubuntu-latest' ],
          runner_id: 1,
          runner_name: 'Hosted Agent',
          runner_group_id: 2,
          runner_group_name: 'GitHub Actions'
        },
        {
          id: 5182943473,
          run_id: 1840752095,
          run_url: 'https://api.github.com/repos/nymtech/nym/actions/runs/1840752095',
          run_attempt: 1,
          node_id: 'CR_kwDODdjOis8AAAABNO1w8Q',
          head_sha: 'aa00eb70d57751bfa556bd3602df87c7473367fc',
          url: 'https://api.github.com/repos/nymtech/nym/actions/jobs/5182943473',
          html_url: 'https://github.com/nymtech/nym/runs/5182943473?check_suite_focus=true',
          status: 'completed',
          conclusion: 'failure',
          started_at: '2022-02-14T11:29:04Z',
          completed_at: '2022-02-14T11:55:45Z',
          name: 'build (macos-latest, stable, schedule)',
          steps: [
            {
              name: 'Set up job',
              status: 'completed',
              conclusion: 'success',
              number: 1,
              started_at: '2022-02-14T13:29:04.000+02:00',
              completed_at: '2022-02-14T13:29:26.000+02:00'
            },
            {
              name: 'Install Dependencies (Linux)',
              status: 'completed',
              conclusion: 'skipped',
              number: 2,
              started_at: '2022-02-14T13:29:26.000+02:00',
              completed_at: '2022-02-14T13:29:26.000+02:00'
            },
            {
              name: 'Keybase - Send Notification',
              status: 'completed',
              conclusion: 'failure',
              number: 15,
              started_at: '2022-02-14T13:55:44.000+02:00',
              completed_at: '2022-02-14T13:55:44.000+02:00'
            },
          ],
          check_run_url: 'https://api.github.com/repos/nymtech/nym/check-runs/5182943473',
          labels: [ 'macos-latest' ],
          runner_id: 4,
          runner_name: 'GitHub Actions 4',
          runner_group_id: 2,
          runner_group_name: 'GitHub Actions'
        },
        ...
      ]
    }
   */

  const jobResults = jobs
    .map((job) => {
      const icon = job.conclusion === 'success' ? '🟩' : '🟥';

      // each job is converted into formatted markdown text
      return `${icon} ${job.conclusion}: ${job.name} - ${job.html_url}`;
    })
    // and join with newlines for display in the template
    .join('\n');

  return template({ ...context, jobResults });
}

module.exports = {
  addToContextAndValidate,
  getMessageBody,
};
