import React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { ServiceProvider } from 'src/types/directory';

export const ServiceProviderInfo = ({ serviceProvider }: { serviceProvider: ServiceProvider }) => (
  <Stack gap={1} sx={{ wordWrap: 'break-word', maxWidth: 200, p: 1 }}>
    <Typography variant="body2" fontWeight="bold">
      Connection info
    </Typography>
    <Divider />
    <Typography variant="caption" fontWeight="bold">
      Gateway <Typography variant="caption">{serviceProvider.gateway}</Typography>
    </Typography>
    <Divider />
    <Typography variant="caption" fontWeight="bold">
      Service provider <Typography variant="caption">{serviceProvider.address.slice(0, 35)}...</Typography>
    </Typography>
  </Stack>
);

export const GatwayWarningInfo = () => (
  <Stack gap={1} sx={{ wordWrap: 'break-word', maxWidth: 200, p: 1 }}>
    <Typography variant="body2" fontWeight="bold" color="warning.main">
      Connection issue
    </Typography>
    <Divider />
    <Typography variant="caption">Try disconnecting and connecting again</Typography>
  </Stack>
);
