# `@nym-project/nym-client-wasm`

This package contains a WASM client for using the Nym mixnet.

## Usage

You should use `@nym-project/sdk` instead of using this package directory.

If you want to use it directly, you'll need to configure bundling and interop as suits your project.

## Build

The build process is a bit nasty. This is necessary to make the package easier to consume.

Build the package by running the following from this directory:

```
scripts/build.sh
```

The following files will be copied into this directory, so that it becomes a package that can be used by the `@nym-project/sdk` package:

```
nym_client_wasm.d.ts
nym_client_wasm.js
nym_client_wasm_bg.wasm
nym_client_wasm_bg.wasm.d.ts
package.json
```

## Publish to `npm`

First build the package with the instructions above.

Publish to `npm` with:

```
npm publish
```