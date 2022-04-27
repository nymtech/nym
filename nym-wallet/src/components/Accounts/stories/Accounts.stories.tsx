import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';

import { v4 as uuid4 } from 'uuid';
import { Accounts } from '../Accounts';

export default {
  title: 'Wallet / Multi Account',
  component: Accounts,
} as ComponentMeta<typeof Accounts>;

const Template: ComponentStory<typeof Accounts> = (args) => (
  <Box display="flex" alignContent="center">
    <Accounts {...args} />
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  accounts: [
    { name: 'Account 1', address: uuid4() },
    { name: 'Account 2', address: uuid4() },
  ],
  selectedAccount: { name: 'Account 1', address: uuid4() },
};
