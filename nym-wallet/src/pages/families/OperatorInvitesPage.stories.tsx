import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { withFamiliesMock } from 'src/components/Families/withFamiliesMock';
import { buildSeededStore, MOCK_OPERATOR_ADDRESS } from 'src/context/mocks/families.fixtures';
import { OperatorInvitesPage } from './OperatorInvitesPage';

const meta: Meta<typeof OperatorInvitesPage> = {
  title: 'Families/Pages/OperatorInvitesPage',
  component: OperatorInvitesPage,
  decorators: [withFamiliesMock({ sender: MOCK_OPERATOR_ADDRESS, makeStore: buildSeededStore })],
};
export default meta;

type Story = StoryObj<typeof OperatorInvitesPage>;

/** Multi-node operator: active invite, expired invite, and a node with none. */
export const MultiNode: Story = {};
