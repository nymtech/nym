import React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { ServiceProvider, Gateway } from 'src/types/directory';

export const ServiceProviderInfo = ({
  serviceProvider,
  gateway,
}: {
  serviceProvider: ServiceProvider;
  gateway: Gateway;
}) => (
  <Stack gap={1} sx={{ wordWrap: 'break-word', maxWidth: 150, p: 1 }}>
    <Typography variant="body2" fontWeight="bold">
      Connection info
    </Typography>
    <Divider />
    <Typography variant="caption" fontWeight="bold">
      Gateway <Typography variant="caption">{gateway.identity}</Typography>
    </Typography>
    <Divider />
    <Typography variant="caption" fontWeight="bold">
      Service provider <Typography variant="caption">{serviceProvider.address.slice(0, 35)}...</Typography>
    </Typography>
  </Stack>
);

export const GatwayWarningInfo = () => (
  <Stack gap={1} sx={{ wordWrap: 'break-word', maxWidth: 150, p: 1 }}>
    <Typography variant="body2" fontWeight="bold" color="warning.main">
      Connection issue
    </Typography>
    <Divider />
    <Typography variant="caption">Try disconnecting and connecting again</Typography>
  </Stack>
);
