import * as React from 'react';
import { Box, Container, Grid, Typography } from '@mui/material';
import { NymLogo } from '@nymproject/react/logo/NymLogo';
import { Playground } from '@nymproject/react/playground/Playground';
import { useIsMounted } from '@nymproject/react/hooks/useIsMounted';
import { NymThemeProvider } from '@nymproject/mui-theme';
import { useTheme } from '@mui/material/styles';
import { ThemeToggle } from './ThemeToggle';
import { AppContextProvider, useAppContext } from './context';

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

  const swatches: Record<string, string> = {
    'palette.primary.main': theme.palette.primary.main,
    'palette.secondary.main': theme.palette.secondary.main,
    'palette.info.main': theme.palette.info.main,
    'palette.success.main': theme.palette.success.main,
    'palette.text.primary': theme.palette.text.primary,
    'theme.palette.nym.networkExplorer.mixnodes.status.active':
      theme.palette.nym.networkExplorer.mixnodes.status.active,
    'theme.palette.nym.networkExplorer.mixnodes.status.standby':
      theme.palette.nym.networkExplorer.mixnodes.status.standby,
  };

  return (
    <Container sx={{ py: 4 }}>
      <Box display="flex" flexDirection="row-reverse" pb={2}>
        <ThemeToggle />
      </Box>
      <NymLogo height={50} />
      <h1>Example App</h1>
      <Box mb={10}>
        <Typography sx={{ color: ({ palette }) => palette.nym.networkExplorer.mixnodes.status.active }}>
          This is an example app that uses React, Typescript, Webpack and the Nym theme + components.
        </Typography>
        <h4>Some colours from the theme (mode = {mode}) are:</h4>
        <Grid container spacing={2}>
          {Object.keys(swatches).map((key) => (
            <Grid item key={key}>
              <Box display="flex" alignItems="center">
                <svg height="50px" width="50px">
                  <rect width="100%" height="100%" fill={swatches[key]} />
                </svg>
                <Typography mx={2}>
                  <code>{swatches[key]}</code>
                  <br />
                  <code>{key}</code>
                </Typography>
              </Box>
            </Grid>
          ))}
        </Grid>
      </Box>
      <h1>Component playground</h1>
      <Playground />
    </Container>
  );
};

export const App: React.FC = () => (
  <AppContextProvider>
    <AppTheme>
      <Content />
    </AppTheme>
  </AppContextProvider>
);
