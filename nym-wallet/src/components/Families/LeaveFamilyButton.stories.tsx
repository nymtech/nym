import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { LeaveFamilyButton } from './LeaveFamilyButton';

const meta: Meta<typeof LeaveFamilyButton> = {
  title: 'Families/Components/LeaveFamilyButton',
  component: LeaveFamilyButton,
  args: { familyName: 'Alpine Routers', onLeave: () => undefined },
};
export default meta;

type Story = StoryObj<typeof LeaveFamilyButton>;

export const Default: Story = {};
export const Busy: Story = { args: { isBusy: true } };
