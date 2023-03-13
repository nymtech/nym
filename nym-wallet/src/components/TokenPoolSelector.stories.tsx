import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { TokenPoolSelector } from './TokenPoolSelector';
import { MockMainContextProvider } from '../context/mocks/main';

export default {
  title: 'Wallet / Token pool',
  component: TokenPoolSelector,
} as ComponentMeta<typeof TokenPoolSelector>;

const Template: ComponentStory<typeof TokenPoolSelector> = (args) => (
  <Box mt={2} height={800}>
    <MockMainContextProvider>
      <TokenPoolSelector {...args} />
    </MockMainContextProvider>
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  disabled: false,
  onSelect: () => {},
};
