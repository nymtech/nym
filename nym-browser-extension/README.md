# Nym Browser Extension

The Nym browser extension lets you access your Nym wallet via the browser.

## Getting started

You will need:

- NodeJS (use `nvm install` to automatically install the correct version)
- `npm`
- `yarn`

> **Note**: This project is part of a mono repo, so you will need to build the shared packages before starting. And any time they change, you'll need to rebuild them.

From the [root of the repository](../README.md) run the following to build shared packages:

```
yarn
yarn build
```

From the `nym-browser-extension` directory of the `nym` monorepo, run:

`yarn dev` to run the extension in dev mode.

You can then open a browser to http://localhost:9000 and start development.

OR

`yarn build` to build the extension.

The extension will build to the `nym-browser-extension/dist` directory.

## Load extension

To load the extension into a Chrome browser

- Go to `settings > extensions > manage extensions`
- Select `Load unpacked`
- Select the `nym-browser-extension/dist`
