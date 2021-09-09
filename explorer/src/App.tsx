import * as React from 'react';
import { CssBaseline, ThemeProvider } from '@material-ui/core';
import { theme } from './theme';
import { Routes } from './routes';

export const App: React.FC = () => (
  <ThemeProvider theme={theme}>
    <CssBaseline />
    <Routes />
  </ThemeProvider>
);
