import React, { useCallback } from 'react';
import { Typography } from '@mui/material';
import { useLocation } from 'react-router';

export const Step = ({ totalSteps }: { totalSteps: number }) => {
  const location = useLocation();

  const mapPage = useCallback(() => {
    switch (location.pathname) {
      case '/create-mnemonic':
        return 1;
      case '/verify-mnemonic':
        return 2;
      case '/create-password':
        return 3;
      default:
        return 0;
    }
  }, [location.pathname]);

  if (mapPage() === 0) {
    return null;
  }
  return (
    <Typography sx={{ color: 'grey.400' }}>
      Create account. Step {mapPage()}/{totalSteps}
    </Typography>
  );
};
