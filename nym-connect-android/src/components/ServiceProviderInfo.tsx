import React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { ServiceProvider } from 'src/types/directory';

export const ServiceProviderInfo = ({ serviceProvider }: { serviceProvider: ServiceProvider }) => (
  <Stack gap={1} sx={{ wordWrap: 'break-word', maxWidth: 150, p: 1 }}>
    <Typography variant="body2" fontWeight="bold">
      Connection info
    </Typography>
    <Typography variant="caption">{serviceProvider.description}</Typography>
    <Divider />
    <Typography variant="caption" fontWeight="bold">
      Gateway <Typography variant="caption">{serviceProvider.gateway}</Typography>
    </Typography>
    <Divider />
    <Typography variant="caption" fontWeight="bold">
      Provider <Typography variant="caption">{serviceProvider.address.slice(0, 35)}...</Typography>
    </Typography>
  </Stack>
);
