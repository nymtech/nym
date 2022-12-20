import React from 'react';
import { Box, Button } from '@mui/material';

export const TestAndEarnError: React.FC<{ error?: string }> = ({ error = 'An error has occurred' }) => (
  <Box>
    <Box mb={4} fontWeight="bold">
      {error}
    </Box>
    <Button variant="outlined" color="secondary">
      Send us an error report
    </Button>
  </Box>
);
