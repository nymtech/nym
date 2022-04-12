import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { ShowMnemonicModal } from './ShowMnemonicModal';

export default {
  title: 'Wallet / Multi Account / Show Mnemonic',
  component: ShowMnemonicModal,
} as ComponentMeta<typeof ShowMnemonicModal>;

const Template: ComponentStory<typeof ShowMnemonicModal> = (args) => (
  <Box display="flex" alignContent="center">
    <ShowMnemonicModal {...args} />
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  mnemonic:
    'lonely employ curtain skull gas swim pizza injury tail birth inmate apart giraffe behave caution hammer echo action best symptom skull toast beyond casino',
  show: true,
  onClose: () => {},
};
