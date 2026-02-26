# GitHub Actions Support Files

This is a collection of scripts and files to support GitHub Actions.

## Sending Notifications

These scripts send CI notifications to Matrix by creating messages from templates and env vars passed from GitHub
Actions.

### Running locally

You will need:

- Node 16 LTS
- npm

Copy `.github/workflows/support-files/.env.example` to `.github/workflows/support-files/.env` and valid Matrix
credentials.

Then run `npm install` to get dependencies.

Start development mode for the notification type you want either by passing the value as an env var called
`NYM_NOTIFICATION_KIND` or set the `.env` file values correctly.

```bash
cd .github/workflows/support-files
npm install
cp .env.example .env
vi .env
npm run dev
```
