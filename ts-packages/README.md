# Nym Typescript (and Javascript) packages

This directory has shared Typescript (and Javascript) libraries using `npm workspaces`.

## Why `ts-packages`?

Naming stuff is hard. The `ts-` part means Typescript, because this monorepo also contains Rust crates. So we needed some way to indicate: "put your Typescript here".

Now you know! So, please, put your Typescript here. And your Javascript.

## How does it work?

In the root of this repository is [package.json](../package.json) that specifies an array of globs for packages that are shared:

```
{
  "name": "@nymproject/nymsphere",
  "version": "1.0.0",
  "private": true,
  "license": "Apache 2.0",
  "workspaces": ["ts-packages/*", "nym-wallet"] <-------
}
```

There are some caveats:

- `npm` creates a single `node_modules` in the root for shared dependencies
- packages that use shared packages, need to be in a path specified in `workspaces`
- local packages take precedence over published packages on `npm`

## Building

From the [root of the repository](../README.md) run:

```
npm install
npm run build
```

This will build all libraries.

Now you can try out [react-webpack-with-theme-example](./react-webpack-with-theme-example) by running:

```
cd ts-packages/react-webpack-with-theme-example
npm run start
```

Our React components have a Storybook in [react-components](./react-components):

```
cd ts-packages/react-components
npm run storybook
```

## Development

Watch mode is available in some packages with `npm run watch`.

See [mui-theme](./mui-theme/README.md) for an example.
