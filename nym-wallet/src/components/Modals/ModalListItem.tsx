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
  /** row: label and value on one line; stack: label above value (better for long strings) */
  layout?: 'row' | 'stack';
}> = ({ label, value, hidden, fontWeight, fontSize, divider, sxValue, layout = 'row' }) => (
  <Box sx={{ display: hidden ? 'none' : 'block' }}>
    {layout === 'stack' ? (
      <Stack spacing={0.5} alignItems="flex-start">
        <Typography
          fontSize="smaller"
          fontWeight={fontWeight ?? 600}
          sx={{ color: 'text.secondary', fontSize: 12, textTransform: 'uppercase', letterSpacing: 0.6 }}
        >
          {label}
        </Typography>
        {value ? (
          <Typography
            fontSize="smaller"
            fontWeight={fontWeight}
            sx={{ color: 'text.primary', fontSize: fontSize || 14, width: '100%', wordBreak: 'break-word', ...sxValue }}
          >
            {value}
          </Typography>
        ) : null}
      </Stack>
    ) : (
      <Stack direction="row" justifyContent="space-between" alignItems="center" gap={1}>
        <Typography fontSize="smaller" fontWeight={fontWeight} sx={{ color: 'text.primary', fontSize: 14 }}>
          {label}
        </Typography>
        {value ? (
          <Typography
            fontSize="smaller"
            fontWeight={fontWeight}
            sx={{ color: 'text.primary', fontSize: fontSize || 14, textAlign: 'right', ...sxValue }}
          >
            {value}
          </Typography>
        ) : null}
      </Stack>
    )}
    {divider && <ModalDivider />}
  </Box>
);
