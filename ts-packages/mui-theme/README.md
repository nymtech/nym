# Nym MUI Theme

This package provides an extension to the MUI theme system to use Nym branding.

If you are unfamiliar with Material UI theming, please read the following first:
- https://mui.com/customization/theming/
- https://mui.com/customization/palette/
- https://mui.com/customization/dark-mode/#dark-mode-with-custom-palette

## Add theme typings to your project

This package also provides a [template file](./template/mui-theme.d.ts) to add typings to the theme using Typescript's module augmentation.

Read the following if you are unfamiliar with module augmentation and declaration merging. Then
look at the recommendations from Material UI docs for implementation:
- https://www.typescriptlang.org/docs/handbook/declaration-merging.html#module-augmentation
- https://www.typescriptlang.org/docs/handbook/declaration-merging.html#merging-interfaces
- https://mui.com/customization/palette/#adding-new-colors

## Example usage

You can see an example of how to use this theme in [react-webpack-with-theme-example](../react-webpack-with-theme-example/src/App.tsx):

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
  <Typography sx={{ color: (theme) => theme.palette.nym.networkExplorer.mixnodes.status.active }}>
    The quick brown fox jumps over the white fence
  </Typography>
  ...
}
```

## Development

The best way to make changes to the theme is to:

1. Run this package in watch mode with `yarn watch`
2. Run Storybook from [react-components](../react-components/README.md) with `yarn storybook`
3. Make sure the component you are changing is included in the [playground](../react-components/src/playground/index.tsx)
4. Watch for changes in the [Playground story](../react-components/src/stories/Playground.stories.tsx)

Also remember to check light mode and dark mode!

## Building

This package should be built from the root of the repository as follows:

```
yarn
yarn build
```

## Publishing

This package is not published to NPM ... yet.
