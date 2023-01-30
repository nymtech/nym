# Information for Nym developers

## Building from source

You will need to have the Rust WASM toolchain installed. Please [follow the instructions here](https://rustwasm.github.io/docs/book/game-of-life/setup.html)
to install `wasm-pack`.

Make sure you have either Node 16 LTS or Node 18 LTS installed and are using it to do the build.

Change the monorepo settings to include the examples directory:

In the `package.json` in the root of the repo, change:
```
  "workspaces": [
    "sdk/typescript/packages/**",
```
to:
```
  "workspaces": [
    "sdk/typescript/**",
```


Run these commands to build the SDK from `sdk/typescript/packages/sdk`:

```
yarn
yarn build:dev
```

The output bundle will be created in the `dist` directory.

## Publishing

If you're a Nym platform developer who's made changes to the Rust (or JS) files and wants to re-publish the package to NPM, here's how you do it:

1. bump version numbers as necessary for SemVer
2. `yarn build` builds the release directory in the root of the repo in `dist/sdk`
3. `cd ../../../../dist/sdk`

Publish the CJS package:
- `cd cjs`
-`npm publish --access=public` will publish your changed package to NPM

Publish the ESM package:
- `cd ../esm`
- `npm publish --access=public` will publish your changed package to NPM
