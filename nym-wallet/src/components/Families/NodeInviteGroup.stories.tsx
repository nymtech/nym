import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { MOCK_NOW_SECS, MOCK_OTHER_OWNER_ADDRESS } from 'src/context/mocks/families.fixtures';
import { NodeInviteGroup } from './NodeInviteGroup';

const meta: Meta<typeof NodeInviteGroup> = {
  title: 'Families/Components/NodeInviteGroup',
  component: NodeInviteGroup,
  args: {
    nodeId: 201,
    nowSecs: MOCK_NOW_SECS,
    onAccept: () => undefined,
    onReject: () => undefined,
    invites: [],
  },
};
export default meta;

type Story = StoryObj<typeof NodeInviteGroup>;

export const Empty: Story = {};
export const WithInvites: Story = {
  args: {
    invites: [
      {
        family_id: 2,
        family_name: 'Alpine Routers',
        owner_address: MOCK_OTHER_OWNER_ADDRESS,
        expires_at: MOCK_NOW_SECS + 7200,
        expired: false,
      },
      {
        family_id: 3,
        family_name: 'Carpathian Mixers',
        owner_address: MOCK_OTHER_OWNER_ADDRESS,
        expires_at: MOCK_NOW_SECS - 1,
        expired: true,
      },
    ],
  },
};
