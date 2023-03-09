import React from 'react';
import { Box } from '@mui/material';
import { useClientContext } from 'src/context/main';

export const AppVersion = () => {
  const { appVersion } = useClientContext();

  return (
    <Box sx={{ display: 'flex', width: '100%', justifyContent: 'center' }}>
      <Box fontSize="small" sx={{ color: 'grey.600' }}>
        Version {appVersion}
      </Box>
    </Box>
  );
};
