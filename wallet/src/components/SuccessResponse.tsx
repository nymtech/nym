import React from 'react';
import { Stack, Typography } from '@mui/material';

export const SuccessReponse: FCWithChildren<{
  title: string;
  subtitle: string | React.ReactNode;
  caption: string | React.ReactNode;
}> = ({ title, subtitle, caption }) => (
  <Stack spacing={3} alignItems="center" sx={{ mb: 5 }}>
    <Typography
      variant="h5"
      fontWeight="600"
      data-testid="transaction-complete"
      color="success.main"
      textTransform="uppercase"
    >
      {title}
    </Typography>
    <Typography fontWeight="600">{subtitle}</Typography>
    <Typography>{caption}</Typography>
  </Stack>
);
