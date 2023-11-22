import * as React from 'react';
import { IconButton, Tooltip } from '@mui/material';
import { SxProps } from '@mui/system';
import { DelegateIcon } from '../../../icons/DelevateSVG';

export const DelegateIconButton: FCWithChildren<{
  disabled?: boolean;
  tooltip?: React.ReactNode;
  sx?: SxProps;
  onDelegate: () => void;
}> = ({ tooltip, onDelegate, sx, disabled }) => {
  const handleOnDelegate = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    onDelegate();
  };
  return (
    <Tooltip title={tooltip || undefined}>
      <IconButton disabled={disabled} onClick={handleOnDelegate} sx={sx} color="primary">
        <DelegateIcon />
      </IconButton>
    </Tooltip>
  );
};
