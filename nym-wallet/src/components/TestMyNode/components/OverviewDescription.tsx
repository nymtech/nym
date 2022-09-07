import React from 'react';
import { Box, Typography } from '@mui/material';

export const OverviewDescription = ({ title, description }: { title: string; description: string }) => (
  <Box>
    <Typography fontWeight="bold" sx={{ mb: 1 }}>
      {title}
    </Typography>
    <Typography fontSize="small" sx={{ color: 'grey.700', mb: 2 }}>
      {description}
    </Typography>
  </Box>
);
