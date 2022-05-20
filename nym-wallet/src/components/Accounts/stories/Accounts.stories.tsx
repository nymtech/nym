import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { MockAccountsProvider } from 'src/context/mocks/accounts';
import { Accounts } from '../Accounts';

export default {
  title: 'Wallet / Multi Account',
  component: Accounts,
} as ComponentMeta<typeof Accounts>;

export const Default: ComponentStory<typeof Accounts> = () => (
  <Box display="flex" alignContent="center">
    <MockAccountsProvider>
      <Accounts />
    </MockAccountsProvider>
  </Box>
);
