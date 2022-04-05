import { NymWalletThemeWithMode } from '../src/theme/NymWalletTheme';

export const parameters = {
  actions: { argTypesRegex: "^on[A-Z].*" },
  controls: {
    matchers: {
      color: /(background|color)$/i,
      date: /Date$/,
    },
  },
}

const withThemeProvider = (Story, context) => (
  <NymWalletThemeWithMode mode="light">
    <Story {...context} />
  </NymWalletThemeWithMode>
);

export const decorators = [withThemeProvider];
