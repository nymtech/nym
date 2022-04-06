import { Box } from '@mui/material';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Accounts } from 'src/components/Accounts';

export default {
  title: 'Wallet / Multi Account',
  component: Accounts,
} as ComponentMeta<typeof Accounts>;

const Template: ComponentStory<typeof Accounts> = (args) => (
  <Box display="flex" alignContent="center">
    <Accounts {...args} />
  </Box>
);

export const Default = Template.bind({});
Default.args = {
  accounts: [
    { name: 'Account 1', address: 'abcdefg' },
    { name: 'Account 2', address: 'tuvwxyz' },
  ],
};
