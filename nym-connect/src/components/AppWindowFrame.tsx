import React from 'react';
import { Box } from '@mui/material';
import { CustomTitleBar } from './CustomTitleBar';

export const AppWindowFrame: React.FC = ({ children }) => (
  <Box
    sx={{
      display: 'grid',
      borderRadius: '12px',
      // screen height is 540px - These should add up to that
      gridTemplateRows: '40px 1fr 30px',
      bgcolor: 'nym.background.dark',
      height: '100vh',
      overflowY: 'hidden',
    }}
  >
    <CustomTitleBar />
    <Box style={{ padding: '16px' }}>{children}</Box>
  </Box>
);
