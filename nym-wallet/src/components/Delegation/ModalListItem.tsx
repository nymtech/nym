import React from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { ModalDivider } from '../Modals/ModalDivider';

export const ModalListItem: React.FC<{
  label: string;
  divider?: boolean;
  hidden?: boolean;
  strong?: boolean;
  value: React.ReactNode;
}> = ({ label, value, hidden, divider, strong }) => (
  <Box sx={{ display: hidden ? 'none' : 'block' }}>
    <Stack direction="row" justifyContent="space-between">
      <Typography fontSize="smaller" fontWeight={strong ? 600 : undefined}>
        {label}:
      </Typography>
      <Typography fontSize="smaller" fontWeight={strong ? 600 : undefined}>
        {value}
      </Typography>
    </Stack>
    {divider && <ModalDivider />}
  </Box>
);
