import React from 'react';
import { Button } from '@mui/material';
import ArrowBackIosNewIcon from '@mui/icons-material/ArrowBackIosNew';

export const StyledBackButton = ({ onBack }: { onBack: () => void }) => (
  <Button disableFocusRipple size="large" variant="outlined" onClick={onBack}>
    <ArrowBackIosNewIcon fontSize="small" />
  </Button>
);
