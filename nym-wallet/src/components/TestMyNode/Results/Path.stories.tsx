import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { Path } from '.';

export default {
  title: 'Test my node / Node path',
  component: Path,
} as ComponentMeta<typeof Path>;

const Template: ComponentStory<typeof Path> = (args) => (
  <Box display="flex">
    <Path {...args} />
  </Box>
);

export const Gateway = Template.bind({});
Gateway.args = {
  layer: 'gateway',
};

export const LayerOne = Template.bind({});
LayerOne.args = {
  layer: '1',
};

export const LayerTwo = Template.bind({});
LayerTwo.args = {
  layer: '2',
};

export const LayerThree = Template.bind({});
LayerThree.args = {
  layer: '3',
};
