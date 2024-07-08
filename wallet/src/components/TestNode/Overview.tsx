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
        <Typography sx={{ color: 'grey.600', pb: 1 }}>
          When you click the Test my node button the Nym Wallet creates a path to the Mixnet which uses a gateway and 3
          nodes. One of the 3 nodes in the path is your node.
        </Typography>
        <Typography sx={{ color: 'grey.600', pb: 1 }}>20 test packets are then sent throught the mixnet.</Typography>
        <Typography sx={{ color: 'grey.600' }}>
          The test results, including the performance score which is the percentage of packets received back through the
          network, are then displayed.
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
