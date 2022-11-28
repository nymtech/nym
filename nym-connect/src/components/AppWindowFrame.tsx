import React from 'react';
import { Box } from '@mui/material';
import { CustomTitleBar } from './CustomTitleBar';

export const AppWindowFrame: React.FC = ({ children }) => (
  <Box
    sx={{
      background: (t) => t.palette.background.default,
      borderRadius: '12px',
      padding: '12px 16px',
      display: 'grid',
      gridTemplateRows: '40px 1fr',
      bgcolor: 'nym.background.dark',
      height: '100vh',
    }}
  >
    <CustomTitleBar />
    <Box style={{ padding: '16px' }}>{children}</Box>
  </Box>
);
