import React from 'react';
import { Box } from '@mui/material';
import { CustomTitleBar } from './CustomTitleBar';
import { AppVersion } from './AppVersion';

export const AppWindowFrame: FCWithChildren = ({ children }) => (
  <Box
    sx={{
      display: 'grid',
      borderRadius: '12px',
      gridTemplateRows: '40px 1fr 30px',
      height: '100vh',
      overflowY: 'hidden',
    }}
  >
    <CustomTitleBar />
    <Box style={{ padding: '16px' }}>{children}</Box>
    <AppVersion />
  </Box>
);
