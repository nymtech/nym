import React from 'react';
import { CheckCircleOutline } from '@mui/icons-material';
import { Card } from '@mui/material';
import { ResultsCardDetail } from './ResultsCardDetail';

export const ResultsCard: React.FC<{ label: string; detail: string; isOk: boolean }> = ({
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
