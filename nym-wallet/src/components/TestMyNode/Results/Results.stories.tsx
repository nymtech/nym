import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { NodeSpeed, Results } from '.';

export default {
  title: 'Test my node / Results',
  component: Results,
} as ComponentMeta<typeof Results>;

const Template: ComponentStory<typeof Results> = (args) => <Results {...args} />;

export const Default = Template.bind({});
Default.args = {
  layer: '1',
  packetsSent: '1000',
  packetsReceived: '5000',
};
