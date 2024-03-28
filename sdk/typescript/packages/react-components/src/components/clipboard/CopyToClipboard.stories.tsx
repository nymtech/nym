import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box, Typography } from '@mui/material';
import { CopyToClipboard } from './CopyToClipboard';

export default {
  title: 'Decorators / Copy to clipboard',
  component: CopyToClipboard,
} as ComponentMeta<typeof CopyToClipboard>;

const Template: ComponentStory<typeof CopyToClipboard> = (args) => {
  const { value } = args;
  return (
    <Box display="flex" alignContent="center">
      <CopyToClipboard {...args} />
      <Typography ml={1}>{value}</Typography>
    </Box>
  );
};
export const Default = Template.bind({});
Default.args = {
  tooltip: 'Copy identity key to clipboard',
  value: '123456',
};

export const SmallIcon = Template.bind({});
SmallIcon.args = {
  tooltip: 'Copy identity key to clipboard',
  value: '123456',
  smallIcons: true,
};
