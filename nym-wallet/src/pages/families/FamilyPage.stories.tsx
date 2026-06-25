import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { within, userEvent } from 'storybook/test';
import { withFamiliesMock } from 'src/components/Families/withFamiliesMock';
import { buildSeededStore, MOCK_OPERATOR_ADDRESS, MOCK_OWNER_ADDRESS } from 'src/context/mocks/families.fixtures';
import { FamilyPage } from './FamilyPage';

const FRESH_ADDRESS = 'n1fresh00000000000000000000000000000fresh';

const meta: Meta<typeof FamilyPage> = {
  title: 'Families/Pages/FamilyPage',
  component: FamilyPage,
};
export default meta;

type Story = StoryObj<typeof FamilyPage>;

/** Account owns a family → management surface. */
export const OwnerWithFamily: Story = {
  decorators: [withFamiliesMock({ sender: MOCK_OWNER_ADDRESS, makeStore: buildSeededStore })],
};

/** Account owns no family → create entry point. */
export const OwnerNoFamily: Story = {
  decorators: [withFamiliesMock({ sender: FRESH_ADDRESS, makeStore: buildSeededStore })],
};

/** Operator persona → switch to the Node invites tab. */
export const Operator: Story = {
  decorators: [withFamiliesMock({ sender: MOCK_OPERATOR_ADDRESS, makeStore: buildSeededStore })],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByTestId('family-tab-operator'));
  },
};
