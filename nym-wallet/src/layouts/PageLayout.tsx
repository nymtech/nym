import React from 'react';
import { Box } from '@mui/material';

export const PageLayout: React.FC<{ position?: 'flex-start' | 'flex-end' }> = ({ position, children }) => (
  <Box
    sx={{
      maxHeight: 'calc(100% - 65px)',
      display: 'flex',
      flexFlow: 'column wrap',
      justifyContent: 'start',
      alignItems: position || 'center',
      overflow: 'auto',
      mt: 2,
      pb: 5,
    }}
  >
    <Box width="100%" margin="auto">
      {children}
    </Box>
  </Box>
);
