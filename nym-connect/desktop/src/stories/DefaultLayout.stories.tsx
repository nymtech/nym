import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { Disconnected } from 'src/pages/connection/Disconnected';
import { ConnectionStatusKind } from '../types';

export default {
  title: 'Layouts/DefaultLayout',
  component: Disconnected,
} as ComponentMeta<typeof Disconnected>;

const onClick = () => undefined;

export const Default: ComponentStory<typeof Disconnected> = () => (
  <Box p={1} width={230} sx={{ bgcolor: 'nym.background.dark' }}>
    <Disconnected
      status={ConnectionStatusKind.disconnected}
      clearError={() => {}}
      error={undefined}
      onConnectClick={onClick}
    />
  </Box>
);

export const WithServices: ComponentStory<typeof Disconnected> = () => (
  <Box p={1} width={230} sx={{ bgcolor: 'nym.background.dark' }}>
    <Disconnected
      status={ConnectionStatusKind.disconnected}
      services={[
        {
          id: '1',
          description: 'Keybase service',
          items: [{ id: '1', description: 'Keybase service 1', address: '123abc' }],
        },
      ]}
      clearError={() => {}}
      error={undefined}
      onConnectClick={onClick}
    />
  </Box>
);
