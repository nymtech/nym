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
  <Box p={4} sx={{ background: 'white' }}>
    <DefaultLayout status={ConnectionStatusKind.disconnected} />
  </Box>
);
