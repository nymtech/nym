import React from 'react';
import { Box, Button, Stack, Typography } from '@mui/material';
import { NymCard } from '../NymCard';

export const Tutorial = () => (
  <Box>
    <Stack direction="row" justifyContent="space-between" sx={{ mb: 3, mt: 2 }}>
      <Box>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>
          How to buy NYM with Bity?
        </Typography>
        <Typography variant="subtitle1">Follow these 3 steps below to quickly and easily buy NYM tokens</Typography>
      </Box>
      <Button variant="contained" color="primary" onClick={() => {}}>
        Buy Nym
      </Button>
    </Stack>
    <Stack direction="row" alignItems="center" justifyContent="space-between" gap={1}>
      <NymCard title="1. Define the purchase amount">Card 1</NymCard>
      <NymCard title="2. Sign a message with your Nym wallet">Card 2</NymCard>
      <NymCard title="3. Transfer funds and receive NYM">Card 3</NymCard>
    </Stack>
  </Box>
);
