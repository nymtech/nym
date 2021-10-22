import { Typography } from '@mui/material';
import React from 'react';

export const ComponentError: React.FC<{ text: string }> = ({ text }) => (
  <Typography
    sx={{ marginTop: 2, color: 'primary.main', fontSize: 10 }}
    variant="body1"
  >
    {text}
  </Typography>
);
