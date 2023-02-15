import React from 'react';
import { Divider, Typography } from '@mui/material';
import { Box } from '@mui/system';

const appsSchema = {
  messagingApps: ['Telegram', 'Keybase'],
  wallets: ['Blockstream', 'Electrum'],
};

export const CompatibleApps = () => (
  <Box>
    <Typography fontWeight={600} sx={{ mb: 3 }}>
      Supported apps
    </Typography>
    <Typography color="nym.highlight" fontWeight={400} sx={{ mb: 2 }}>
      Messaging apps
    </Typography>

    <Box sx={{ mb: 3 }}>
      {appsSchema.messagingApps.map((app) => (
        <Typography variant="body2" color="grey.400" sx={{ mb: 2 }} key={app}>
          {app}
        </Typography>
      ))}
    </Box>
    <Divider sx={{ mb: 3 }} />
    <Typography color="nym.highlight" fontWeight={400} sx={{ mb: 2 }}>
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
