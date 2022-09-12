import React from 'react';
import { Button, SxProps } from '@mui/material';
import ArrowBackIosNewIcon from '@mui/icons-material/ArrowBackIosNew';

export const StyledBackButton = ({
  onBack,
  label,
  fullWidth,
  sx,
}: {
  onBack: () => void;
  label?: string;
  fullWidth?: boolean;
  sx?: SxProps;
}) => (
  <Button disableFocusRipple size="large" fullWidth={fullWidth} variant="outlined" onClick={onBack} sx={sx}>
    {label || <ArrowBackIosNewIcon fontSize="small" />}
  </Button>
);
