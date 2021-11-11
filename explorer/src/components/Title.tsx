import * as React from 'react';
import { Typography } from '@mui/material';

export const Title: React.FC<{ text: string }> = ({ text }) => (
  <Typography
    variant="h5"
    sx={{
      mb: 3,
    }}
    data-testid={text}
  >
    {text}
  </Typography>
);
