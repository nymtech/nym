import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { AddAccountModal } from 'src/components/Accounts/AddAccountModal';

export default {
  title: 'Wallet / Multi Account / Add Account',
  component: AddAccountModal,
} as ComponentMeta<typeof AddAccountModal>;

const Template: ComponentStory<typeof AddAccountModal> = () => (
  <Box display="flex" alignContent="center">
    <AddAccountModal />
  </Box>
);

export const Default = Template.bind({});
Default.args = {};

export const WithoutPassword = Template.bind({});
WithoutPassword.args = {
  withoutPassword: true,
};
