import React from 'react';
import { Box, Stack, Typography, TypographyProps } from '@mui/material';
import { ModalDivider } from './ModalDivider';

export const ModalListItem: FCWithChildren<{
  label: string;
  divider?: boolean;
  hidden?: boolean;
  fontWeight?: TypographyProps['fontWeight'];
  light?: boolean;
  value?: React.ReactNode;
}> = ({ label, value, hidden, fontWeight, divider }) => (
  <Box sx={{ display: hidden ? 'none' : 'block' }}>
    <Stack direction="row" justifyContent="space-between">
      <Typography fontSize="smaller" fontWeight={fontWeight} sx={{ color: 'text.primary', fontSize: 14 }}>
        {label}
      </Typography>
      {value && (
        <Typography fontSize="smaller" fontWeight={fontWeight} sx={{ color: 'text.primary', fontSize: 14 }}>
          {value}
        </Typography>
      )}
    </Stack>
    {divider && <ModalDivider />}
  </Box>
);
