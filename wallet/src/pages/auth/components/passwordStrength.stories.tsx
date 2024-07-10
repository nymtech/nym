import { useState } from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Stack, TextField } from '@mui/material';
import { PasswordStrength } from './password-strength';

export default {
  title: 'Wallet / Password Strength',
  component: PasswordStrength,
} as ComponentMeta<typeof PasswordStrength>;

const Template: ComponentStory<typeof PasswordStrength> = ({ password, withWarnings, handleIsSafePassword }: any) => {
  const [value, setValue] = useState(password);
  return (
    <Stack alignContent="center">
      <TextField value={value} onChange={(e) => setValue(e.target.value)} sx={{ mb: 0.5 }} />
      {!!password.length && (
        <PasswordStrength handleIsSafePassword={handleIsSafePassword} withWarnings={withWarnings} password={password} />
      )}
    </Stack>
  );
};

export const VeryStrong = Template.bind({});
VeryStrong.args = { password: 'fedgklnrf34£', withWarnings: true, handleIsSafePassword: () => undefined };

export const Strong = Template.bind({});
Strong.args = { password: '"56%abc123?@', withWarnings: true, handleIsSafePassword: () => undefined };

export const Average = Template.bind({});
Average.args = { password: '"abc123?', withWarnings: true, handleIsSafePassword: () => undefined };

export const Weak = Template.bind({});
Weak.args = { password: 'abc123?', withWarnings: true, handleIsSafePassword: () => undefined };

export const VeryWeak = Template.bind({});
VeryWeak.args = {
  password: 'abc123',
  withWarnings: true,
  handleIsSafePassword: () => undefined,
};

export const WithName = Template.bind({});
WithName.args = {
  password: 'fred',
  withWarnings: true,
  handleIsSafePassword: () => undefined,
};

export const WithSequence = Template.bind({});
WithSequence.args = {
  password: '121212',
  withWarnings: true,
  handleIsSafePassword: () => undefined,
};

export const Default = Template.bind({});
Default.args = {
  password: 'abc123',
  withWarnings: true,
  handleIsSafePassword: () => undefined,
};
