import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { ConnectionStats } from '../components/ConnectionStats';

export default {
  title: 'Components/ConnectionStats',
  component: ConnectionStats,
} as ComponentMeta<typeof ConnectionStats>;

const Template: ComponentStory<typeof ConnectionStats> = (args) => <ConnectionStats {...args} />;

// 👇 Each story then reuses that template
export const Default = Template.bind({});
Default.args = {
  stats: [
    {
      label: 'in:',
      totalBytes: 1024,
      rateBytesPerSecond: 1024 * 1024 * 1024 + 10,
    },
    {
      label: 'out:',
      totalBytes: 1024 * 1024 * 1024 * 1024 * 20,
      rateBytesPerSecond: 1024 * 1024 + 10,
    },
  ],
};
