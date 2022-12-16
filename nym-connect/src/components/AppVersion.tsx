import { Typography, Box } from '@mui/material';
import React from 'react';
import { useClientContext } from 'src/context/main';

export const AppVersion = () => {
  const { appVersion } = useClientContext();

  return (
    <Box sx={{ display: 'grid', width: '100%', justifyContent: 'center' }}>
      <Box fontSize="small" sx={{ mb: 4, color: 'grey.600' }}>
        Version {appVersion}
      </Box>
    </Box>
  );
};
