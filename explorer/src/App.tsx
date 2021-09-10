import * as React from 'react';
import { CssBaseline } from '@mui/material';
import { ThemeProvider } from '@mui/styles';

import { theme } from './theme';
import { Routes } from './routes';

export const App: React.FC = () => (
  <ThemeProvider theme={theme}>
    <CssBaseline />
    <Routes />
  </ThemeProvider>
);
