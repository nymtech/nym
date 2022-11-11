import React from 'react';
import { Box } from '@mui/material';
import { CustomTitleBar } from './CustomTitleBar';

export const AppWindowFrame: React.FC = ({ children }) => (
  <Box
    style={{
      display: 'grid',
      gridTemplateRows: '40px 1fr',
      background: '#1D2125',

      height: '100vh',
    }}
  >
    <CustomTitleBar />
    <Box style={{ padding: '16px' }}>{children}</Box>
  </Box>
);
