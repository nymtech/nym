# MixFetch Internal Tester

This project is for use by Nym developers only. Use at your own risk!

## Getting started

From the root of this repository run:

```
yarn
make sdk-wasm-build
```

Then:

```
cd sdk/typescript/packages/mix-fetch
```

You can run in watch mode:

```
yarn start
```

Or do a single build:

```
yarn build:dev:esm-no-inline
```

Then, in another terminal:

```
cd sdk/typescript/packages/mix-fetch/internal-dev/parcel
yarn && yarn start
```

If you have trouble with changes not propagating:

```
rm -rf node_modules && yarn && yarn start
```

