import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { InviteNodeForm } from './InviteNodeForm';

const meta: Meta<typeof InviteNodeForm> = {
  title: 'Families/Components/InviteNodeForm',
  component: InviteNodeForm,
  args: { onSubmit: () => undefined },
};
export default meta;

type Story = StoryObj<typeof InviteNodeForm>;

export const Default: Story = {};
export const WarningAlreadyInFamily: Story = { args: { warning: 'already-in-family' } };
export const WarningNonExistent: Story = { args: { warning: 'non-existent' } };
export const WarningDuplicatePending: Story = { args: { warning: 'duplicate-pending' } };
export const Submitting: Story = { args: { isSubmitting: true } };
export const ContractError: Story = { args: { errorMessage: 'Something went wrong.' } };
