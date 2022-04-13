import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { AddAccountModal } from 'src/components/Accounts/AddAccountModal';

export default {
  title: 'Wallet / Multi Account / Add Account',
  component: AddAccountModal,
} as ComponentMeta<typeof AddAccountModal>;

const Template: ComponentStory<typeof AddAccountModal> = (args) => (
  <Box display="flex" alignContent="center">
    <AddAccountModal {...args} />
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  show: true,
  onClose: () => {},
  onAdd: () => {},
};

export const WithoutPassword = Template.bind({});
WithoutPassword.args = {
  show: true,
  withoutPassword: true,
  onClose: () => {},
  onAdd: () => {},
};
