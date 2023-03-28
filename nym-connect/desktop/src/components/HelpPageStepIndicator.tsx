import React from 'react';
import { Box } from '@mui/material';

const Step = ({ highlight }: { highlight: boolean }) => (
  <Box sx={{ width: '65px', height: '1px', bgcolor: highlight ? 'nym.highlight' : 'grey.600' }} />
);

export const StepIndicator = ({ step }: { step: number }) => (
  <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
    <Step highlight />
    <Step highlight={step >= 2} />
    <Step highlight={step >= 3} />
  </Box>
);
