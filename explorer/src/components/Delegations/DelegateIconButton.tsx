import * as React from 'react';
import { Button } from '@mui/material';
import { SxProps } from '@mui/system';

export const DelegateIconButton: FCWithChildren<{
  size?: 'small' | 'medium';
  disabled?: boolean;
  tooltip?: React.ReactNode;
  sx?: SxProps;
  onDelegate: () => void;
}> = ({ onDelegate, sx, disabled, size = 'medium' }) => {
  const handleOnDelegate = () => {
    onDelegate();
  };
  return (
    <Button variant="outlined" size={size} disabled={disabled} onClick={handleOnDelegate} sx={sx}>
      Delegate
    </Button>
  );
};
