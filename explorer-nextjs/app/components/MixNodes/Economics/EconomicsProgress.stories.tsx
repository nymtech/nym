import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { EconomicsProgress } from './EconomicsProgress';

export default {
  title: 'Mix Node Detail/Economics/ProgressBar',
  component: EconomicsProgress,
} as ComponentMeta<typeof EconomicsProgress>;

const Template: ComponentStory<typeof EconomicsProgress> = (args) => <EconomicsProgress {...args} />;

export const Empty = Template.bind({});
Empty.args = {};

export const OverThreshold = Template.bind({});
OverThreshold.args = {
  threshold: 100,
  value: 120,
};

export const UnderThreshold = Template.bind({});
UnderThreshold.args = {
  threshold: 100,
  value: 80,
};

export const OnThreshold = Template.bind({});
OnThreshold.args = {
  threshold: 100,
  value: 100,
};
