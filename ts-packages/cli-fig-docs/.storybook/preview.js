/* eslint-disable react/react-in-jsx-scope */
import { NymThemeProvider } from '@nymproject/mui-theme';
import { Box } from '@mui/material';

export const parameters = {
  actions: { argTypesRegex: '^on[A-Z].*' },
  controls: {
    matchers: {
      color: /(background|color)$/i,
      date: /Date$/,
    },
  },
};

const withThemeProvider = (Story, context) => (
  <div style={{ display: 'grid', height: '100%', gridTemplateColumns: '50% 50%' }}>
    <div>
      <NymThemeProvider mode="light">
        <Box
          p={4}
          sx={{
            display: 'grid',
            gridTemplateRows: '80vh 2rem',
            background: (theme) => theme.palette.background.default,
            color: (theme) => theme.palette.text.primary,
          }}
        >
          <Box sx={{ overflowY: 'auto' }}>
            <Story {...context} />
          </Box>
          <h4 style={{ textAlign: 'center' }}>Light mode</h4>
        </Box>
      </NymThemeProvider>
    </div>
    <div>
      <NymThemeProvider mode="dark">
        <Box
          p={4}
          sx={{
            display: 'grid',
            gridTemplateRows: '80vh 2rem',
            background: (theme) => theme.palette.background.default,
            color: (theme) => theme.palette.text.primary,
          }}
        >
          <Box sx={{ overflowY: 'auto' }}>
            <Story {...context} />
          </Box>
          <h4 style={{ textAlign: 'center' }}>Dark mode</h4>
        </Box>
      </NymThemeProvider>
    </div>
  </div>
);

export const decorators = [withThemeProvider];
