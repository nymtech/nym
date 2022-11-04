import React from 'react';
import { Box } from '@mui/material';
import { CustomTitleBar } from './CustomTitleBar';

export const AppWindowFrame: React.FC = ({ children }) => (
  <Box style={{ background: '#1D2125', borderRadius: '12px', height: '100vh' }}>
    <CustomTitleBar />
    <Box style={{ padding: '16px' }}>{children}</Box>
  </Box>
);
