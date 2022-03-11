import React from 'react';
import { Box, Typography } from '@mui/material';

export const Title: React.FC<{ title: string | React.ReactNode; Icon: any }> = ({ title, Icon }) => (
  <Box display="flex" alignItems="center">
    {Icon && <Icon sx={{ mr: 1 }} />}{' '}
    <Typography variant="h6" sx={{ fontWeight: 600 }}>
      {title}
    </Typography>
  </Box>
);
