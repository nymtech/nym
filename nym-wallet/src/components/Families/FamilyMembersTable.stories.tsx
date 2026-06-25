import type { Meta, StoryObj } from '@storybook/react-webpack5';
import { FamilyMemberSections, PendingMemberRow } from 'src/types/families';
import { MOCK_NOW_SECS } from 'src/context/mocks/families.fixtures';
import { FamilyMembersTable } from './FamilyMembersTable';

const emptySections: FamilyMemberSections = { pending: [], joined: [], rejected: [], removed: [] };

const populatedSections: FamilyMemberSections = {
  pending: [],
  joined: [
    { section: 'joined', node_id: 101, joined_at: MOCK_NOW_SECS },
    { section: 'joined', node_id: 102, joined_at: MOCK_NOW_SECS },
  ],
  rejected: [{ section: 'rejected', node_id: 105, rejected_at: MOCK_NOW_SECS }],
  removed: [{ section: 'removed', node_id: 103, removed_at: MOCK_NOW_SECS }],
};

const pending: PendingMemberRow[] = [
  { section: 'pending', node_id: 107, expires_at: MOCK_NOW_SECS + 3600, expired: false },
  { section: 'pending', node_id: 108, expires_at: MOCK_NOW_SECS - 1, expired: true },
];

const meta: Meta<typeof FamilyMembersTable> = {
  title: 'Families/Components/FamilyMembersTable',
  component: FamilyMembersTable,
  args: {
    sections: emptySections,
    pending: [],
    nowSecs: MOCK_NOW_SECS,
    onKick: () => undefined,
    onRevoke: () => undefined,
    onClearExpired: () => undefined,
    onRefresh: () => undefined,
  },
};
export default meta;

type Story = StoryObj<typeof FamilyMembersTable>;

export const Empty: Story = {};
export const Loading: Story = { args: { isLoading: true } };
export const ErrorState: Story = { args: { isError: true } };
export const Populated: Story = { args: { sections: populatedSections, pending } };
