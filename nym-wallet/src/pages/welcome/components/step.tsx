import React, { useCallback } from 'react';
import { Typography } from '@mui/material';
import { TPages } from '../types';

export const Step = ({ currentPage, totalSteps }: { currentPage: TPages; totalSteps: number }) => {
  const mapPage = useCallback(() => {
    switch (currentPage) {
      case 'create mnemonic':
        return 1;
      case 'verify mnemonic':
        return 2;
      case 'create password':
        return 3;
      default:
        return 0;
    }
  }, [currentPage]);

  if (mapPage() === 0) {
    return null;
  }
  return (
    <Typography sx={{ color: 'grey.400' }}>
      Create account. Step {mapPage()}/{totalSteps}
    </Typography>
  );
};
