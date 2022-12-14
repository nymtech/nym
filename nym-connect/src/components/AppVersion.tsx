import { Typography } from '@mui/material';
import React from 'react';
import { useClientContext } from 'src/context/main';

export const AppVersion = () => {
  const { appVersion } = useClientContext();

  return (
    <Typography fontSize="small" textAlign="center" sx={{ color: 'grey.600' }}>
      Version {appVersion}
    </Typography>
  );
};
