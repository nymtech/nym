import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { ClientAddress } from '../../../lib/components/client-address/ClientAddress';

export default {
  title: 'Wallet / Client Address',
  component: ClientAddress,
} as ComponentMeta<typeof ClientAddress>;

const Template: ComponentStory<typeof ClientAddress> = (args) => (
  <Box display="flex" alignContent="center">
    <ClientAddress {...args} />
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
  smallIcons: true,
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

export const WithSmallIcons = Template.bind({});
WithSmallIcons.args = {
  address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
  withCopy: true,
  smallIcons: true,
};
