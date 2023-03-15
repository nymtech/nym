import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { MockMainContextProvider } from '../context/mocks/main';
import { NetworkSelector } from './NetworkSelector';

export default {
  title: 'Wallet / Network Selector',
  component: NetworkSelector,
} as ComponentMeta<typeof NetworkSelector>;

const Template: ComponentStory<typeof NetworkSelector> = () => (
  <Box mt={2} height={800}>
    <MockMainContextProvider>
      <NetworkSelector />
    </MockMainContextProvider>
  </Box>
);

export const Default = Template.bind({});
