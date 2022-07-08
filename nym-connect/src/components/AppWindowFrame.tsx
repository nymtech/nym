import React from 'react';
import { Box } from '@mui/material';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';

export const AppWindowFrame: React.FC = ({ children }) => (
  <Box
    sx={{
      background: '#121726',
      borderRadius: '12px',
      padding: '12px 16px',
      display: 'grid',
      gridTemplateRows: '30px auto',
      width: '240px',
    }}
  >
    <Box display="flex" justifyContent="space-between" alignItems="center">
      <NymWordmark width={22} />
    </Box>
    {children}
  </Box>
);
