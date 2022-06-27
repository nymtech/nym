import * as React from 'react';
import { Box, Container, Grid, Typography } from '@mui/material';
import { NymLogo } from '@nymproject/react/logo/NymLogo';
import { Playground } from '@nymproject/react/playground/Playground';
import { useIsMounted } from '@nymproject/react/hooks/useIsMounted';
import { NymThemeProvider } from '@nymproject/mui-theme';
import { useTheme } from '@mui/material/styles';
import { ThemeToggle } from './ThemeToggle';
import { AppContextProvider, useAppContext } from './context';
import { MixNodes } from './components/MixNodes';

export const AppTheme: React.FC = ({ children }) => {
  const { mode } = useAppContext();

  return <NymThemeProvider mode={mode}>{children}</NymThemeProvider>;
};

export const Content: React.FC = () => {
  const { mode } = useAppContext();
  const theme = useTheme();
  const isMounted = useIsMounted();

  if (isMounted()) {
    console.log('Content is mounted');
  }

  return (
    <Box sx={{ px: 4, py: 4 }}>
      <Box display="flex" justifyContent="space-between" pb={2}>
        <Box display="flex" alignItems="center">
          <NymLogo height={50} />
          <Box ml={2}>
            <h1>APY Playground</h1>
          </Box>
        </Box>
        <Box>
          <ThemeToggle />
        </Box>
      </Box>
      <MixNodes />
    </Box>
  );
};

export const App: React.FC = () => (
  <AppContextProvider>
    <AppTheme>
      <Content />
    </AppTheme>
  </AppContextProvider>
);
