import React from 'react';
import { CheckCircleOutline } from '@mui/icons-material';
import { Stack, Typography, Box, Card } from '@mui/material';

export const ResultsCardDetail = ({
  label,
  detail,
  DescriptionIcon,
}: {
  label: string | React.ReactNode;
  detail: string;
  DescriptionIcon?: React.ReactNode;
}) => (
  <Stack direction="row" justifyContent="space-between">
    {label}
    <Box display="flex" gap={1} alignItems="center">
      <Typography>{detail}</Typography>
      {DescriptionIcon}
    </Box>
  </Stack>
);

export const ResultsCard: React.FC<{
  label: string | React.ReactNode;
  detail: string;
  showTick?: boolean;
  children: React.ReactNode;
}> = ({ label, detail, showTick, children }) => (
  <Card variant="outlined" sx={{ p: 3, height: '100%' }}>
    <ResultsCardDetail
      label={label}
      detail={detail}
      DescriptionIcon={showTick && <CheckCircleOutline sx={{ color: 'success.light' }} />}
    />
    {children}
  </Card>
);
