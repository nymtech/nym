import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { DateTime } from 'luxon';
import { Connected } from 'src/pages/connection/Connected';
import { ConnectionStatusKind } from 'src/types';

const onClick = () => undefined;

export default {
  title: 'Layouts/ConnectedLayout',
  component: Connected,
} as ComponentMeta<typeof Connected>;

export const Default: ComponentStory<typeof Connected> = () => (
  <Box p={2} width={242} sx={{ bgcolor: 'nym.background.dark' }}>
    <Connected
      clearError={() => {}}
      gatewayPerformance="Good"
      showInfoModal={false}
      closeInfoModal={() => undefined}
      status={ConnectionStatusKind.connected}
      connectedSince={DateTime.now()}
      ipAddress="127.0.0.1"
      serviceProvider={{ id: 'service 1', description: 'good services', address: 'abc123' }}
      port={1080}
      stats={[
        {
          label: 'in:',
          totalBytes: 1024,
          rateBytesPerSecond: 1024 * 1024 * 1024 + 10,
        },
        {
          label: 'out:',
          totalBytes: 1024 * 1024 * 1024 * 1024 * 20,
          rateBytesPerSecond: 1024 * 1024 + 10,
        },
      ]}
      onConnectClick={onClick}
    />
  </Box>
);
