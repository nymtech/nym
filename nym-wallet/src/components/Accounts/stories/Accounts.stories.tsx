import React from 'react';
import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Accounts } from '../Accounts';

export default {
  title: 'Wallet / Multi Account',
  component: Accounts,
} as ComponentMeta<typeof Accounts>;

const Template: ComponentStory<typeof Accounts> = () => (
  <Box display="flex" alignContent="center">
    <Accounts />
  </Box>
);

export const Default = Template.bind({});
Default.args = {};
