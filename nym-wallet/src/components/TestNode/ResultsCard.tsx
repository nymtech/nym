import React from 'react';
import { Stack, Typography, Card } from '@mui/material';

export const ResultsCardDetail = ({
  label,
  detail,
  largeText,
}: {
  label: string | React.ReactNode;
  detail: string;
  largeText?: boolean;
}) => (
  <Stack direction="row" justifyContent="space-between">
    <Typography variant={largeText ? 'h6' : 'body1'}>{label}</Typography>
    <Typography variant={largeText ? 'h6' : 'body1'}>{detail}</Typography>
  </Stack>
);

export const ResultsCard: React.FC<{
  label: string | React.ReactNode;
  detail: string;
  children: React.ReactNode;
}> = ({ label, detail, children }) => (
  <Card variant="outlined" sx={{ p: 3, height: '100%' }}>
    <ResultsCardDetail label={label} detail={detail} />
    {children}
  </Card>
);
