# Example with React + Typescript + Webpack 5 + MUI

An example of using default Webpack and Typescript settings with React and MUI, including theming.

You can use this example as a seed for a new project.

Remember to build the dependency packages from the root of this repo by running:

```
yarn
yarn build
```

If you need to make changes to the dependency packages, you can run `yarn watch` in that package to watch for chagnes and build them. This project will pick up the changes in the built package and hot-reload / recompile.

## Features

### Yarn workspaces

Packages from `ts-packages` are shared using Yarn workspaces. Make sure you add you new project to [package.json](../../package.json) to use the shared packages.

> ⚠️ **Warning**: Yarn workspaces will share all dependencies between projects and works by falling back to parent directories until a `node_modules` directory is found. So be careful when messing around with `node_modules` and resolution, because unexpected things could happen - for example, if you do not run `yarn` from the root and you have a `node_modules` in a directory that is a parent of the directory where you checkout out this repository, that `node_modules` will be used for resolving packages 🙀.

### Typescript

Shared Typescript config is in [tsconfig.json](./tsconfig.json), with specific production settings in [tsconfig.prod.json](./tsconfig.prod.json) that:

- exclude Storybook stories and Jest tests
- do not output typing `*.d.ts` files

### Webpack

Inherit config for Webpack 5 with additional tweaks including:

- favicon generation from [favicon asset files](../../assets/favicon/favicon.png)
- asset handling (svg, png, fonts, css, etc)
- minification

The development settings include:

- `ts-loader` for quick transpilation
- threaded type checking using `tsc`
- hot reloading using `react-refresh`

### Storybook

Storybook is available in [@nymproject/react](../react-components/src/stories/Introduction.stories.mdx) and can be run using `yarn storybook`.

### MUI and theming

The [Nym theme](../mui-theme/src/theme/theme.ts) provides a theme provider that you can add as follows:

```typescript jsx
export const App: React.FC = () => (
  <AppContextProvider>
    <AppTheme>
      <Content />
    </AppTheme>
  </AppContextProvider>
);

export const AppTheme: React.FC = ({ children }) => {
  const { mode } = useAppContext();

  return <NymThemeProvider mode={mode}>{children}</NymThemeProvider>;
};

export const Content: React.FC = () => {
...
}
```

And augment typings for the Theme by adding [mui-theme.d.ts](./src/theme/mui-theme.d.ts):

```typescript
import { Theme, ThemeOptions, Palette, PaletteOptions } from '@mui/material/styles';
import { NymTheme, NymPaletteWithExtensions, NymPaletteWithExtensionsOptions } from '@nymproject/mui-theme';

declare module '@mui/material/styles' {
  interface Theme extends NymTheme {}
  interface ThemeOptions extends Partial<NymTheme> {}
  interface Palette extends NymPaletteWithExtensions {}
  interface PaletteOptions extends NymPaletteWithExtensionsOptions {}
}
```

Adding the above, means that any component now has the correct typings, for example, below the Nym palette interface is available for all MUI `Theme` instances with code completion for VSCode and IntelliJ:

```typescript jsx
import { Typography } from '@mui/material';

...

<Typography sx={{ color: (theme) => theme.palette.nym.networkExplorer.mixnodes.status.active }}>
  The quick brown fox jumps over the white fence
</Typography>

```
