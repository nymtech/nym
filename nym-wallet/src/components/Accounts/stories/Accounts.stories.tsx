import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';

import { v4 as uuid4 } from 'uuid';
import { AccountsContainer } from '../AccountContainer';

export default {
  title: 'Wallet / Multi Account',
  component: AccountsContainer,
} as ComponentMeta<typeof AccountsContainer>;

const Template: ComponentStory<typeof AccountsContainer> = (args) => (
  <Box display="flex" alignContent="center">
    <AccountsContainer {...args} />
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  storedAccounts: [
    { name: 'Account 1', address: uuid4() },
    { name: 'Account 2', address: uuid4() },
  ],
};
