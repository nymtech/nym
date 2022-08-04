import React from 'react';
import { Box, Typography } from '@mui/material';

export const Title: React.FC<{ title: string | React.ReactNode; Icon?: React.ReactNode }> = ({ title, Icon }) => (
  <Box width="100%" display="flex" alignItems="center">
    {Icon}
    <Typography width="100%" variant="h5" sx={{ fontWeight: 600 }}>
      {title}
    </Typography>
  </Box>
);
