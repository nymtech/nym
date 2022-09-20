import React from 'react';
import { Button, SxProps } from '@mui/material';
import ArrowBackIosNewIcon from '@mui/icons-material/ArrowBackIosNew';

export const StyledBackButton = ({ onBack, sx }: { onBack: () => void; sx?: SxProps }) => (
  <Button disableFocusRipple size="large" variant="outlined" onClick={onBack} sx={sx}>
    <ArrowBackIosNewIcon fontSize="small" />
  </Button>
);
