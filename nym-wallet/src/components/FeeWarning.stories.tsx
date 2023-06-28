import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { BalanceWarning } from './FeeWarning';

export default {
  title: 'Wallet / Balance warning',
  component: BalanceWarning,
} as ComponentMeta<typeof BalanceWarning>;

const Template: ComponentStory<typeof BalanceWarning> = (args) => (
  <Box mt={2} height={800}>
    <BalanceWarning {...args} />
  </Box>
);

export const WithWarning = Template.bind({});
WithWarning.args = {
  fee: '200',
};
