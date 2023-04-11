# GitHub Actions Support Files

This is a collection of scripts and files to support GitHub Actions.

## Sending Notifications

These scripts send CI notifications to Keybase by creating messages from templates and env vars passed from GitHub Actions.

### Adding notifications to a GitHub Action

```
jobs:
  build:
    ...
    - name: Notifications - Node Install
      run: npm install
      working-directory: .github/workflows/support-files/notifications
    - name: Notifications - Send
      env:
        NYM_NOTIFICATION_KIND: "my-component"
        GIT_BRANCH: "${GITHUB_REF##*/}"
        KEYBASE_NYMBOT_USERNAME: "${{ secrets.KEYBASE_NYMBOT_USERNAME }}"
        KEYBASE_NYMBOT_PAPERKEY: "${{ secrets.KEYBASE_NYMBOT_PAPERKEY }}"
        KEYBASE_NYMBOT_TEAM: "${{ secrets.KEYBASE_NYMBOT_TEAM }}"
        KEYBASE_NYM_CHANNEL: "ci-network-explorer"
        IS_SUCCESS: "${{ job.status == 'success' }}"
      uses: docker://keybaseio/client:stable-node
      with:
        args: .github/workflows/support-files/notifications/entry_point.sh
```

Notifications are run by adding the snippet above to a GitHub Action, and:

1. Installing node packages needed at run time
2. Set the env vars as required:
    - `NYM_NOTIFICATION_KIND` matches the directory in `.github/workflows/support-files/${NYM_NOTIFICATION_KIND}` to provide the templates and extra scripting in `index.js`
    - Keybase credentials, channel and other env vars for the status of the build and repo
3. Replacing the default entry point shell script on the `keybaseio/client:stable-node` docker image to run `.github/workflows/support-files/notifications/entry_point.sh` 

### Running locally

You will need:
- Node 16 LTS
- npm

Copy `.github/workflows/support-files/.env.example` to `.github/workflows/support-files/.env` and valid Keybase credentials.

Then run `npm install` to get dependencies.

Start development mode for the notification type you want either by passing the value as an env var called `NYM_NOTIFICATION_KIND` or set the `.env` file values correctly.

```bash
cd .github/workflows/support-files
npm install
cp .env.example .env
vi .env
npm run dev
```