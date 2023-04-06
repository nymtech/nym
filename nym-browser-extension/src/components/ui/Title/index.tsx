import React from 'react';
import { Typography } from '@mui/material';

const FONT_SIZE = 22;
const FONT_WEIGHT = 400;

export const Title = ({ children }: { children: string }) => (
  <Typography variant="h5" fontWeight={FONT_WEIGHT}>
    {children}
  </Typography>
);
