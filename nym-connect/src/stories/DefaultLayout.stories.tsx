import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { DefaultLayout } from '../layouts/DefaultLayout';
import { ConnectionStatusKind } from '../types';

export default {
  title: 'Layouts/DefaultLayout',
  component: DefaultLayout,
} as ComponentMeta<typeof DefaultLayout>;

export const Default: ComponentStory<typeof DefaultLayout> = () => (
  <Box p={1} width={230} sx={{ bgcolor: 'nym.background.dark' }}>
    <DefaultLayout status={ConnectionStatusKind.disconnected} clearError={() => {}} error={undefined} />
  </Box>
);

export const WithServices: ComponentStory<typeof DefaultLayout> = () => (
  <Box p={1} width={230} sx={{ bgcolor: 'nym.background.dark' }}>
    <DefaultLayout
      status={ConnectionStatusKind.disconnected}
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
