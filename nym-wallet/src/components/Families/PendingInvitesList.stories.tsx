import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { MOCK_NOW_SECS } from 'src/context/mocks/families.fixtures';
import { PendingInvitesList } from './PendingInvitesList';

const meta: Meta<typeof PendingInvitesList> = {
  title: 'Families/Components/PendingInvitesList',
  component: PendingInvitesList,
  args: {
    nowSecs: MOCK_NOW_SECS,
    onRevoke: () => undefined,
    onClearExpired: () => undefined,
  },
};
export default meta;

type Story = StoryObj<typeof PendingInvitesList>;

export const Empty: Story = { args: { invites: [] } };
export const ActiveAndExpired: Story = {
  args: {
    invites: [
      { section: 'pending', node_id: 107, expires_at: MOCK_NOW_SECS + 3600, expired: false },
      { section: 'pending', node_id: 108, expires_at: MOCK_NOW_SECS - 1, expired: true },
    ],
  },
};
