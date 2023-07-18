import React from 'react';
import { Divider, Typography } from '@mui/material';
import { Box } from '@mui/system';

const appsSchema = {
  messagingApps: ['Matrix', 'Telegram', 'Keybase'],
  wallets: ['Monero', 'Blockstream', 'Electrum', 'Alephium'],
};

export const CompatibleApps = () => (
  <Box>
    <Typography fontWeight="bold" variant="body2" sx={{ mb: 2 }}>
      Supported apps
    </Typography>
    <Typography color="nym.highlight" sx={{ mb: 2 }}>
      Messaging apps
    </Typography>

    <Box sx={{ mb: 2 }}>
      {appsSchema.messagingApps.map((app) => (
        <Typography variant="body2" color="grey.400" sx={{ mb: 2 }} key={app}>
          {app}
        </Typography>
      ))}
    </Box>
    <Divider sx={{ mb: 2 }} />
    <Typography color="nym.highlight" sx={{ mb: 2 }}>
      Wallets
    </Typography>

    <Box sx={{ mb: 4 }}>
      {appsSchema.wallets.map((wallet) => (
        <Typography variant="body2" color="grey.400" sx={{ mb: 2 }} key={wallet}>
          {wallet}
        </Typography>
      ))}
    </Box>
  </Box>
);
