import React from 'react';
import { Box } from '@mui/material';
import { ContentRailWidth, resolveContentRailMaxWidth } from './contentRail';

export const PageLayout: FCWithChildren<{
  position?: 'flex-start' | 'flex-end';
  maxWidth?: ContentRailWidth;
  children: React.ReactNode;
}> = ({ position, maxWidth, children }) => (
  <Box
    sx={{
      minHeight: '100%',
      display: 'flex',
      flexDirection: 'column',
      justifyContent: 'flex-start',
      alignItems: position || 'stretch',
      mt: { xs: 2, md: 3 },
    }}
  >
    <Box width="100%" maxWidth={resolveContentRailMaxWidth(maxWidth)} mx="auto">
      {children}
    </Box>
  </Box>
);
