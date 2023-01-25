import React from 'react';
import { Box } from '@mui/material';
import { CustomTitleBar } from './CustomTitleBar';
import { useLocation, useParams } from 'react-router-dom';

export const AppWindowFrame: FCWithChildren = ({ children }) => {
  const location = useLocation();

  return (
    <Box
      sx={{
        display: 'grid',
        borderRadius: '12px',
        gridTemplateRows: '40px 1fr',
        height: '100vh',
      }}
    >
      <CustomTitleBar path={location.pathname} />
      <Box style={{ padding: '16px' }}>{children}</Box>
    </Box>
  );
};
