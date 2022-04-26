import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { ImportAccountModal } from 'src/components/Accounts/ImportAccountModal';

export default {
  title: 'Wallet / Multi Account / Import Account',
  component: ImportAccountModal,
} as ComponentMeta<typeof ImportAccountModal>;

const Template: ComponentStory<typeof ImportAccountModal> = (args) => (
  <Box display="flex" alignContent="center">
    <ImportAccountModal {...args} />
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  show: true,
  onClose: () => {},
  onImport: () => {},
};
