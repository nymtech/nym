import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { Packets } from '.';

export default {
  title: 'Test my node / Packets',
  component: Packets,
} as ComponentMeta<typeof Packets>;

const Transfer: ComponentStory<typeof Packets> = (args) => (
  <Box width="500px">
    <Packets {...args} />
  </Box>
);

export const HighTransfer = Transfer.bind({});
HighTransfer.args = {
  sent: '100',
  received: '80',
};

export const LowTransfer = Transfer.bind({});
LowTransfer.args = {
  sent: '100',
  received: '50',
};
