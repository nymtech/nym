import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { NodeSpeed } from '.';

export default {
  title: 'Test my node / Node speed',
  component: NodeSpeed,
} as ComponentMeta<typeof NodeSpeed>;

const Template: ComponentStory<typeof NodeSpeed> = (args) => (
  <Box display="flex" alignContent="center">
    <NodeSpeed {...args} />
  </Box>
);

export const FastNode = Template.bind({});
FastNode.args = {
  Mbps: 500,
  performance: 'good',
};

export const FairNode = Template.bind({});
FairNode.args = {
  Mbps: 100,
  performance: 'fair',
};

export const SlowNode = Template.bind({});
SlowNode.args = {
  Mbps: 10,
  performance: 'poor',
};
