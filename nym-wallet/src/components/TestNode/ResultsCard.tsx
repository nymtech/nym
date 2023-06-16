import React from 'react';
import { CheckCircleOutline } from '@mui/icons-material';
import { Stack, Typography, Box, Card } from '@mui/material';

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

export const ResultsCard: React.FC<{ label: string; detail: string; isOk?: boolean; children: React.ReactNode }> = ({
  label,
  detail,
  isOk,
  children,
}) => (
  <Card variant="outlined" sx={{ p: 3 }}>
    <ResultsCardDetail
      label={label}
      detail={detail}
      boldLabel
      DescriptionIcon={isOk && <CheckCircleOutline sx={{ color: 'success.light' }} />}
    />
    {children}
  </Card>
);
