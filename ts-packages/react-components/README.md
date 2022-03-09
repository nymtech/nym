# Nym Shared React Components

This package contains shared React components that are used in other Nym projects.

It uses the following packages:

- [shared MUI theme](../mui-theme/README.md)
- [webpack config](../webpack/README.md)
- [MUI](https://https://mui.com/)
- Typescript
- React

## Building

```
yarn
yarn build
```

## Development

Run watch mode with:

```
yarn watch
```

Or you can run Storybook with:

```
yarn storybook
```

Or you can run the [example project](../react-webpack-with-theme-example/README.md) in dev mode and this package in watch mode, and test results in the example project's dev server output.

## Playground

There are [playground components](./src/playground/index.tsx) that are intended to be used during development to see the effects are changes to the MUI theme or shared components at a glance.

They are available in Storybook from [src/stories/Playground.stories.tsx](./src/stories/Playground.stories.tsx).

> ℹ️ **Tip**: use the playground to make sure components stay consistent and you don't break other components while making changes

## Shared assets

This project uses [shared asset files](../../assets/README.md) such as favicons and logos.

> ℹ️ **Tip**: use to keep your project consistent with Nym's branding and so that it automatically receives changes when the shared assets change. Please try to avoid duplicating the files in the shared assets directory.

## Publishing

This package is not published to NPM ... yet.
