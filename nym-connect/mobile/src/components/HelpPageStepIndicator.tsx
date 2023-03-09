import React from 'react';
import { Box, Stack } from '@mui/material';

const Step = ({ highlight }: { highlight: boolean }) => (
  <Box sx={{ width: '65px', height: '3px', bgcolor: highlight ? 'nym.highlight' : 'grey.600' }} />
);

export const StepIndicator = ({ step }: { step: number }) => (
  <Stack direction="row" alignItems="center" justifyContent="space-between" width="240px">
    <Step highlight />
    <Step highlight={step >= 2} />
    <Step highlight={step >= 3} />
  </Stack>
);
