import React from 'react';
import { Box, Button, Stack, Typography } from '@mui/material';
import { ResultsCard } from './ResultsCard';

export const Overview = ({ onStartTest }: { onStartTest: () => void }) => (
  <ResultsCard
    label={
      <Typography variant="h6" fontWeight="bold">
        Test my node
      </Typography>
    }
    detail=""
  >
    <Stack justifyContent="space-between" sx={{ py: 3, height: '100%' }}>
      <Box>
        <Typography sx={{ color: 'grey.600', pb: 1 }}>Click the test my node button to test your node.</Typography>
        <Typography sx={{ color: 'grey.600' }}>
          The test will send some packets through your node to see how many are sent and received
        </Typography>
      </Box>
      <Box display="flex">
        <Button variant="contained" disableElevation onClick={onStartTest}>
          Test my node
        </Button>
      </Box>
    </Stack>
  </ResultsCard>
);
