import React from 'react';
import { Box } from '@mui/material';
import { CustomTitleBar } from './CustomTitleBar';
import { AppVersion } from './AppVersion';

export const AppWindowFrame: React.FC = ({ children }) => (
  <Box
    sx={{
      display: 'grid',
      borderRadius: '12px',
      // screen height is 540px - These should add up to that
      gridTemplateRows: '40px 470px 30px',
      bgcolor: 'nym.background.dark',
      height: '100vh',
      gridtemplateAreas: '"." "." "."',
    }}
  >
    <CustomTitleBar />
    <Box style={{ padding: '16px' }}>{children}</Box>
    <AppVersion />
  </Box>
);
