import { NymThemeProvider } from '@nymproject/mui-theme';

export const parameters = {
  actions: { argTypesRegex: "^on[A-Z].*" },
  controls: {
    matchers: {
      color: /(background|color)$/i,
      date: /Date$/,
    },
  },
}

export const globalTypes = {
  theme: {
    name: 'Theme',
    description: 'Global theme for components',
    defaultValue: 'light',
    toolbar: {
      icon: 'circlehollow',
      // Array of plain string values or MenuItem shape (see below)
      items: ['light', 'dark'],
      // Property that specifies if the name of the item will be displayed
      showName: true,
    },
  },
};

const withThemeProvider=(Story,context)=>{
  return (
    <NymThemeProvider mode={context.globals.theme}>
      <Story {...context} />
    </NymThemeProvider>
  )
}
export const decorators = [withThemeProvider];
