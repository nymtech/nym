import React from 'react';
import { Box, Button, Typography } from '@mui/material';
import { NymCard } from '../NymCard';

export const Bond = ({
  onBondMixnode,
  onBondGateway,
  disabled,
}: {
  onBondMixnode: () => void;
  onBondGateway: () => void;
  disabled: boolean;
}) => (
  <NymCard title="Bonding">
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
      }}
    >
      <Typography>Bond a mixnode or a gateway</Typography>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'flex-end',
          justifyContent: 'space-between',
          gap: 2,
        }}
      >
        <Button
          size="large"
          variant="contained"
          color="primary"
          type="button"
          disableElevation
          onClick={onBondMixnode}
          disabled={disabled}
        >
          Mixnode
        </Button>
        <Button size="large" variant="contained" onClick={onBondGateway} disabled={disabled}>
          Gateway
        </Button>
      </Box>
    </Box>
  </NymCard>
);
