import { NymMixnetTheme } from '../src/theme';
import { Fonts } from './preview-fonts';
import { MockProvider } from '../src/context/mocks/main';
const withThemeProvider = (Story, context) => {
  return (
    <Fonts>
      <MockProvider>
        <NymMixnetTheme mode="dark">
          <Story {...context} />
        </NymMixnetTheme>
      </MockProvider>
    </Fonts>
  );
};

export const decorators = [withThemeProvider];

export const parameters = {
  actions: { argTypesRegex: '^on[A-Z].*' },
  controls: {
    matchers: {
      color: /(background|color)$/i,
      date: /Date$/,
    },
  },
};
