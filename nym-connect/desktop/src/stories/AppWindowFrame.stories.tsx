import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { AppWindowFrame } from '../components/AppWindowFrame';

export default {
  title: 'App/AppWindowFrame',
  component: AppWindowFrame,
} as ComponentMeta<typeof AppWindowFrame>;

export const Default: ComponentStory<typeof AppWindowFrame> = () => (
  <Box p={4} sx={{ background: 'white' }}>
    <AppWindowFrame>
      Culpa deserunt cupidatat culpa nisi aute dolore nisi deserunt cillum consequat elit. Nostrud id occaecat
      consectetur consectetur excepteur labore consectetur. Laboris tempor consequat qui exercitation adipisicing sunt
      cupidatat est. Officia dolore qui eu dolor velit ex ea qui laborum. Mollit ut est sit irure elit ad ut deserunt.
    </AppWindowFrame>
  </Box>
);
