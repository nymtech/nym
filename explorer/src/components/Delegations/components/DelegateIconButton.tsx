import * as React from 'react';
import { useClipboard } from 'use-clipboard-copy';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import DoneIcon from '@mui/icons-material/Done';
import { Box, Tooltip } from '@mui/material';
import { SxProps } from '@mui/system';
import { DelegateSVG } from '../../../icons/DelevateSVG';

export const DelegateIconButton: FCWithChildren<{
  tooltip?: React.ReactNode;
  onDelegate: () => void;
  sx?: SxProps;
}> = ({ tooltip, onDelegate, sx }) => {
  const handleDelegateClick = () => {
    onDelegate();
  };
  return (
    <Tooltip title={tooltip || undefined}>
      <Box sx={sx} onClick={handleDelegateClick}>
        <DelegateSVG />
      </Box>
    </Tooltip>
  );
};
