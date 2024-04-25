# nym-hash-release

This is the source code for the custom GitHub Action to calculate hashes.

It is in a subdirectory to avoid issues with `package.json`.

## Build

The following will bundle all code and dependencies into the `dist` folder, and copy it into place for GitHub Actions.

```
npm run build
npm run dist:copy
```