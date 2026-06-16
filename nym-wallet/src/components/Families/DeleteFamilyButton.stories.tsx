import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { DeleteFamilyButton } from './DeleteFamilyButton';

const meta: Meta<typeof DeleteFamilyButton> = {
  title: 'Families/Components/DeleteFamilyButton',
  component: DeleteFamilyButton,
  args: { memberCount: 0, onDelete: () => undefined },
};
export default meta;

type Story = StoryObj<typeof DeleteFamilyButton>;

export const Deletable: Story = {};
export const BlockedNonEmpty: Story = { args: { memberCount: 3 } };
export const WithError: Story = { args: { errorMessage: 'The family must be empty before it can be deleted.' } };
