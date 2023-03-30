import React from 'react';
import { Typography } from '@mui/material';
import { NymBrowserExtThemeWithMode } from './theme/NymBrowserExtensionTheme';

export const App = () => (
  <NymBrowserExtThemeWithMode mode="dark">
    <Typography p={4} fontWeight="bold">
      Nym browser extension
    </Typography>
  </NymBrowserExtThemeWithMode>
);
