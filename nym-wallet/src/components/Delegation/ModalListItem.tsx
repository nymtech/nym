import React from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { ModalDivider } from '../Modals/ModalDivider';

export const ModalListItem: React.FC<{
  label: string;
  divider?: boolean;
  hidden?: boolean;
  value: React.ReactNode;
}> = ({ label, value, hidden, divider }) => (
  <Box sx={{ display: hidden ? 'none' : 'block' }}>
    <Stack direction="row" justifyContent="space-between">
      <Typography fontSize="smaller">{label}:</Typography>
      <Typography fontSize="smaller">{value}</Typography>
    </Stack>
    {divider && <ModalDivider />}
  </Box>
);
