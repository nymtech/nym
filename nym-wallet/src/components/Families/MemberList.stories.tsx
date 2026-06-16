import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { FamilyMemberSections } from 'src/types/families';
import { MOCK_NOW_SECS } from 'src/context/mocks/families.fixtures';
import { MemberList } from './MemberList';

const emptySections: FamilyMemberSections = { pending: [], joined: [], rejected: [], removed: [] };

const populatedSections: FamilyMemberSections = {
  pending: [
    { section: 'pending', node_id: 107, expires_at: MOCK_NOW_SECS + 3600, expired: false },
    { section: 'pending', node_id: 108, expires_at: MOCK_NOW_SECS - 1, expired: true },
  ],
  joined: [
    { section: 'joined', node_id: 101, joined_at: MOCK_NOW_SECS },
    { section: 'joined', node_id: 102, joined_at: MOCK_NOW_SECS },
  ],
  rejected: [{ section: 'rejected', node_id: 105, rejected_at: MOCK_NOW_SECS }],
  removed: [
    { section: 'removed', node_id: 103, removed_at: MOCK_NOW_SECS },
    { section: 'removed', node_id: 104, removed_at: MOCK_NOW_SECS },
  ],
};

const meta: Meta<typeof MemberList> = {
  title: 'Families/Components/MemberList',
  component: MemberList,
  args: {
    nowSecs: MOCK_NOW_SECS,
    sections: emptySections,
    onKick: () => undefined,
    onRefresh: () => undefined,
  },
};
export default meta;

type Story = StoryObj<typeof MemberList>;

export const Loading: Story = { args: { isLoading: true } };
export const ErrorState: Story = { args: { isError: true } };
export const Empty: Story = {};
export const Populated: Story = { args: { sections: populatedSections } };
