import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { CreateFamilyForm } from './CreateFamilyForm';

const meta: Meta<typeof CreateFamilyForm> = {
  title: 'Families/Components/CreateFamilyForm',
  component: CreateFamilyForm,
  args: {
    fee: { denom: 'nym', amount: '100' },
    nameLimit: 30,
    descriptionLimit: 120,
    onSubmit: () => undefined,
  },
};
export default meta;

type Story = StoryObj<typeof CreateFamilyForm>;

export const Default: Story = {};
export const InsufficientBalance: Story = { args: { balance: { denom: 'nym', amount: '5' } } };
export const Submitting: Story = { args: { isSubmitting: true } };
export const ContractError: Story = { args: { errorMessage: 'The attached creation fee is incorrect.' } };
