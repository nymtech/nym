import { Box } from '@mui/material';
import React from 'react';

const Step = ({ highlight }: { highlight: boolean }) => (
  <Box sx={{ width: '48px', height: '1px', bgcolor: highlight ? 'nym.highlight' : 'grey.600' }} />
);

export const StepIndicator = ({ step }: { step: number }) => (
  <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-evenly' }}>
    <Step highlight />
    <Step highlight={step >= 2} />
    <Step highlight={step >= 3} />
    <Step highlight={step >= 4} />
  </Box>
);
