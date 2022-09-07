import React from 'react';
import { Box, Stack, Typography } from '@mui/material';

export const ResultsCardDetail = ({
  label,
  detail,
  DescriptionIcon,
  boldLabel,
}: {
  label: string;
  detail: string;
  DescriptionIcon?: React.ReactNode;
  boldLabel?: boolean;
}) => (
  <Stack direction="row" justifyContent="space-between">
    <Typography fontWeight={boldLabel ? 'bold' : 'regular'}>{label}</Typography>
    <Box display="flex" gap={1} alignItems="center">
      <Typography>{detail}</Typography>
      {DescriptionIcon}
    </Box>
  </Stack>
);
