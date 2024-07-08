import { NymWalletThemeWithMode } from '../src/theme/NymWalletTheme';
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
      <NymWalletThemeWithMode mode="light">
        <Box
          p={4}
          sx={{
            display: 'grid',
            gridTemplateRows: '80vh 2rem',
            background: (theme) => theme.palette.background.default,
            color: 'text.primary',
          }}
        >
          <Box sx={{ overflowY: 'auto' }}>
            <Story {...context} />
          </Box>
          <h4 style={{ textAlign: 'center' }}>Light mode</h4>
        </Box>
      </NymWalletThemeWithMode>
    </div>
    <div>
      <NymWalletThemeWithMode mode="dark">
        <Box
          p={4}
          sx={{
            display: 'grid',
            gridTemplateRows: '80vh 2rem',
            background: (theme) => theme.palette.background.default,
            color: 'text.primary',
          }}
        >
          <Box sx={{ overflowY: 'auto' }}>
            <Story {...context} />
          </Box>
          <h4 style={{ textAlign: 'center' }}>Dark mode</h4>
        </Box>
      </NymWalletThemeWithMode>
    </div>
  </div>
);

export const decorators = [withThemeProvider];
