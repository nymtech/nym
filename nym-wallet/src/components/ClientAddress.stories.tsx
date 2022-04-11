import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box, Typography } from '@mui/material';
import { ClientAddressDisplay } from './ClientAddress';

export default {
  title: 'Wallet / Client Address',
  component: ClientAddressDisplay,
} as ComponentMeta<typeof ClientAddressDisplay>;

const Template: ComponentStory<typeof ClientAddressDisplay> = (args) => (
  <Box display="flex" alignContent="center">
    <ClientAddressDisplay {...args} />
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
};

export const WithCopy = Template.bind({});
WithCopy.args = {
  address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
  withCopy: true,
};

export const WithLabel = Template.bind({});
WithLabel.args = {
  withLabel: true,
  address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
};

export const ShowEntireAddress = Template.bind({});
ShowEntireAddress.args = {
  withLabel: true,
  showEntireAddress: true,
  address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
};

export const Empty = Template.bind({});
Empty.args = {};

export const EmptyWithLabelAndCopy = Template.bind({});
EmptyWithLabelAndCopy.args = {
  withLabel: true,
  withCopy: true,
};
