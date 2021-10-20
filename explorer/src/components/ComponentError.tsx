import { Typography } from '@mui/material';
import React from 'react';

type ErrorProps = {
  text: string;
};

export const ComponentError = ({ text }: ErrorProps) => (
  <Typography
    sx={{ marginTop: 2, color: 'primary.main', fontSize: 10 }}
    variant="body1"
  >
    {text}
  </Typography>
);
