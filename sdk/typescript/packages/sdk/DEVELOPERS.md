# Information for Nym developers

## Building from source

You will need to have the Rust WASM toolchain installed. Please [follow the instructions here](https://rustwasm.github.io/docs/book/game-of-life/setup.html)
to install `wasm-pack`. Make sure you also have `wasm-opt` installed, in case it was not installed with `wasm-pack`.

Make sure you have either Node 16 LTS or Node 18 LTS installed and are using it to do the build.

From the **root of the monorepo** (aka `home/path/to/nym/`) run:

```
yarn dev:on
yarn build:wasm
```

Then `cd` here.

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

Make sure you have logged into the NPM registry (`npm login`).

### Scripted

1. bump version numbers as necessary for SemVer
2. make sure you're in the SDK directory (`cd sdk/typescript/packages/sdk`)
3. run `scripts/publish.sh`

### Manually

1. bump version numbers as necessary for SemVer
2. `yarn build` builds the release directory in the root of the repo in `dist/sdk`
3. `cd ../../../../dist/sdk`

Publish the CJS package:
- `cd cjs`
-`npm publish --access=public` will publish your changed package to NPM

Publish the ESM package:
- `cd ../esm`
- `npm publish --access=public` will publish your changed package to NPM

4. bump the minor version and add `-rc0` to avoid local packages from using the workspace SDK, e.g. (`1.1.10` -> `1.1.11-rc0`)
5. commit the updated `package.json` to git
