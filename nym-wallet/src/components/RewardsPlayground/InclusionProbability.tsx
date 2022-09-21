import React from 'react';
import { Typography } from '@mui/material';
import { SelectionChance } from '@nymproject/types';

const colorMap: { [key in SelectionChance]: string } = {
  VeryLow: 'error.main',
  Low: 'error.main',
  Moderate: 'warning.main',
  High: 'success.main',
  VeryHigh: 'success.main',
};

const textMap: { [key in SelectionChance]: string } = {
  VeryLow: 'VeryLow',
  Low: 'Low',
  Moderate: 'Moderate',
  High: 'High',
  VeryHigh: 'Very high',
};

export const InclusionProbability = ({ probability }: { probability: SelectionChance }) => (
  <Typography sx={{ color: colorMap[probability] }}>{textMap[probability]}</Typography>
);
