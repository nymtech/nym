import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { EditFamilyForm } from './EditFamilyForm';

const meta: Meta<typeof EditFamilyForm> = {
  title: 'Families/Components/EditFamilyForm',
  component: EditFamilyForm,
  args: {
    initialName: 'Tatry Operators',
    initialDescription: 'Operators coordinating routing in the Tatra mountains.',
    nameLimit: 30,
    descriptionLimit: 120,
    onSubmit: () => undefined,
  },
};
export default meta;

type Story = StoryObj<typeof EditFamilyForm>;

export const Default: Story = {};
export const OverLimitName: Story = {
  args: { initialName: 'This family name is far too long to be accepted on chain' },
};
export const Submitting: Story = { args: { isSubmitting: true, initialName: 'Tatry Operators v2' } };
export const ContractError: Story = { args: { errorMessage: 'That family name is already taken.' } };
