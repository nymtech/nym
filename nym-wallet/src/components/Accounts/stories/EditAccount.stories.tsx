import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { v4 as uuid } from 'uuid';
import { EditAccountModal } from 'src/components/Accounts/EditAccountModal';

export default {
  title: 'Wallet / Multi Account / Edit Account',
  component: EditAccountModal,
} as ComponentMeta<typeof EditAccountModal>;

const Template: ComponentStory<typeof EditAccountModal> = () => (
  <Box display="flex" alignContent="center">
    <EditAccountModal />
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  account: { id: 'Account 1', address: uuid() },
  show: true,
  onClose: () => {},
};
