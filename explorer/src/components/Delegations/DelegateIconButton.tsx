import * as React from 'react';
import { Button, IconButton } from '@mui/material';
import { SxProps } from '@mui/system';
import { useIsMobile } from '@src/hooks';
import { DelegateIcon } from '@src/icons/DelevateSVG';

export const DelegateIconButton: FCWithChildren<{
  size?: 'small' | 'medium';
  disabled?: boolean;
  tooltip?: React.ReactNode;
  sx?: SxProps;
  onDelegate: () => void;
}> = ({ onDelegate, sx, disabled, size = 'medium' }) => {
  const isMobile = useIsMobile();

  const handleOnDelegate = () => {
    onDelegate();
  };

  if (isMobile) {
    return (
      <IconButton size="small" disabled={disabled} onClick={handleOnDelegate}>
        <DelegateIcon fontSize="small" />
      </IconButton>
    );
  }

  return (
    <Button variant="outlined" size={size} disabled={disabled} onClick={handleOnDelegate} sx={sx}>
      Delegate
    </Button>
  );
};
