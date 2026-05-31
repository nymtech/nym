import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { MOCK_NOW_SECS, MOCK_OTHER_OWNER_ADDRESS } from 'src/context/mocks/families.fixtures';
import { InviteCard } from './InviteCard';

const meta: Meta<typeof InviteCard> = {
  title: 'Families/Components/InviteCard',
  component: InviteCard,
  args: {
    nowSecs: MOCK_NOW_SECS,
    onAccept: () => undefined,
    onReject: () => undefined,
    invite: {
      family_id: 2,
      family_name: 'Alpine Routers',
      owner_address: MOCK_OTHER_OWNER_ADDRESS,
      expires_at: MOCK_NOW_SECS + 7200,
      expired: false,
    },
  },
};
export default meta;

type Story = StoryObj<typeof InviteCard>;

export const Active: Story = {};
export const Expired: Story = {
  args: {
    invite: {
      family_id: 2,
      family_name: 'Alpine Routers',
      owner_address: MOCK_OTHER_OWNER_ADDRESS,
      expires_at: MOCK_NOW_SECS - 1,
      expired: true,
    },
  },
};
