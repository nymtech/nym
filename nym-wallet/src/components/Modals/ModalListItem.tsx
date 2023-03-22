import React from 'react';
import { Box, Stack, SxProps, Typography, TypographyProps } from '@mui/material';
import { ModalDivider } from './ModalDivider';

export const ModalListItem: FCWithChildren<{
  label: string;
  divider?: boolean;
  hidden?: boolean;
  fontWeight?: TypographyProps['fontWeight'];
  fontSize?: TypographyProps['fontSize'];
  light?: boolean;
  value?: React.ReactNode;
  sxValue?: SxProps;
}> = ({ label, value, hidden, fontWeight, fontSize, divider, sxValue }) => (
  <Box sx={{ display: hidden ? 'none' : 'block' }}>
    <Stack direction="row" justifyContent="space-between" alignItems="center">
      <Typography fontSize="smaller" fontWeight={fontWeight} sx={{ color: 'text.primary', fontSize: 14 }}>
        {label}
      </Typography>
      {value && (
        <Typography
          fontSize="smaller"
          fontWeight={fontWeight}
          sx={{ color: 'text.primary', fontSize: fontSize || 14, ...sxValue }}
        >
          {value}
        </Typography>
      )}
    </Stack>
    {divider && <ModalDivider />}
  </Box>
);
