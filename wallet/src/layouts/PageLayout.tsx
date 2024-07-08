import React from 'react';
import { Box } from '@mui/material';

export const PageLayout: FCWithChildren<{ position?: 'flex-start' | 'flex-end'; children: React.ReactNode }> = ({
  position,
  children,
}) => (
  <Box
    sx={{
      maxHeight: 'calc(100% - 65px)',
      display: 'flex',
      flexFlow: 'column wrap',
      justifyContent: 'start',
      alignItems: position || 'center',
      overflow: 'auto',
      mt: 4,
    }}
  >
    <Box width="100%" margin="auto">
      {children}
    </Box>
  </Box>
);
