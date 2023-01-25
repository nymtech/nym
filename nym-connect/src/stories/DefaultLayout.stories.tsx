import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { ConnectionStatusKind } from '../types';
import { Disconnected } from 'src/pages/connection/Disconnected';

export default {
  title: 'Layouts/DefaultLayout',
  component: Disconnected,
} as ComponentMeta<typeof Disconnected>;

export const Default: ComponentStory<typeof Disconnected> = () => (
  <Box p={1} width={230} sx={{ bgcolor: 'nym.background.dark' }}>
    <Disconnected status={'disconnected'} clearError={() => {}} error={undefined} />
  </Box>
);

export const WithServices: ComponentStory<typeof Disconnected> = () => (
  <Box p={1} width={230} sx={{ bgcolor: 'nym.background.dark' }}>
    <Disconnected
      status={'disconnected'}
      services={[
        {
          id: '1',
          description: 'Keybase service',
          items: [{ id: '1', description: 'Keybase service 1', gateway: 'abc123', address: '123abc' }],
        },
      ]}
      clearError={() => {}}
      error={undefined}
    />
  </Box>
);
