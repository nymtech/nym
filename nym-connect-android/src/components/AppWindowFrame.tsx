import React from 'react';
import { Box } from '@mui/material';
import { useLocation } from 'react-router-dom';
import { CustomTitleBar } from './CustomTitleBar';

export const AppWindowFrame: FCWithChildren = ({ children }) => {
  const location = useLocation();

  return (
    <Box
      sx={{
        display: 'grid',
        gridTemplateRows: '40px 1fr',
        height: '100vh',
      }}
    >
      <CustomTitleBar path={location.pathname} />
      <Box style={{ padding: '16px' }}>{children}</Box>
    </Box>
  );
};
