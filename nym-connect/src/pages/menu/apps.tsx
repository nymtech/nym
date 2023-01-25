import React from 'react';
import { Divider, Typography } from '@mui/material';
import { Box } from '@mui/system';

const appsSchema = {
  messagingApps: ['Telegram', 'Keybase'],
  wallets: ['Blockstream', 'Electum'],
};

export const CompatibleApps = () => (
  <Box>
    <Typography fontSize="small" color="grey.600" sx={{ mb: 2 }}>
      Supported apps
    </Typography>
    <Typography color="nym.highlight" sx={{ mb: 2 }}>
      Messaging apps
    </Typography>

    <Divider sx={{ mb: 2 }} />

    <Box sx={{ mb: 4 }}>
      {appsSchema.messagingApps.map((app) => (
        <Typography variant="body2" color="grey.400" sx={{ mb: 2 }}>
          {app}
        </Typography>
      ))}
    </Box>
    <Typography color="nym.highlight" sx={{ mb: 2 }}>
      Wallets
    </Typography>

    <Divider sx={{ mb: 2 }} />

    <Box sx={{ mb: 4 }}>
      {appsSchema.wallets.map((wallet) => (
        <Typography variant="body2" color="grey.400" sx={{ mb: 2 }}>
          {wallet}
        </Typography>
      ))}
    </Box>
  </Box>
);
