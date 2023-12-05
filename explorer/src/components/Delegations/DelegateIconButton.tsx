import * as React from 'react';
import { IconButton, Tooltip } from '@mui/material';
import { SxProps } from '@mui/system';
import { DelegateIcon } from '../../icons/DelevateSVG';

export const DelegateIconButton: FCWithChildren<{
  size?: 'small' | 'medium';
  disabled?: boolean;
  tooltip?: React.ReactNode;
  sx?: SxProps;
  onDelegate: () => void;
}> = ({ tooltip, onDelegate, sx, disabled, size = 'medium' }) => {
  const handleOnDelegate = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    onDelegate();
  };
  return (
    <Tooltip title={tooltip || undefined}>
      <IconButton size={size} disabled={disabled} onClick={handleOnDelegate} sx={sx}>
        <DelegateIcon fontSize={size} />
      </IconButton>
    </Tooltip>
  );
};
