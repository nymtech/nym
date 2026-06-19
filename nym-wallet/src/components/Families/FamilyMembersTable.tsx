/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import {
  Box,
  Button,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material';
import { FamilyMemberSections, MemberRow, PendingMemberRow } from 'src/types/families';
import { ConfirmActionButton } from './ConfirmActionButton';
import { StatusChip } from './StatusChip';
import { formatExpiry } from './helpers';

export interface FamilyMembersTableProps {
  /** Joined / rejected / removed sections (the `pending` field here is unused, see the `pending` prop). */
  sections: FamilyMemberSections;
  /** Pending invites resolved from the live invitations query. */
  pending: PendingMemberRow[];
  nowSecs: number;
  isLoading?: boolean;
  isError?: boolean;
  /** Busy flag for the kick (Remove) action. */
  kicking?: boolean;
  /** Busy flag for the withdraw/clear (revoke) action. */
  revoking?: boolean;
  onKick: (nodeId: number) => void;
  onRevoke: (nodeId: number) => void;
  onClearExpired: (nodeId: number) => void;
  onRefresh: () => void;
}

const NODE_COL_WIDTH = '28%';
const STATUS_COL_WIDTH = '42%';

const cellSx = { py: 1.5, verticalAlign: 'middle' } as const;

const NodeCell = ({ id }: { id: number }) => (
  <TableCell sx={cellSx}>
    <Typography variant="body2" sx={{ fontFamily: 'monospace' }}>
      Node {id}
    </Typography>
  </TableCell>
);

/** One table row per family member/invite record, keyed and status-typed for clarity. */
const MemberTableRow = ({
  row,
  nowSecs,
  kicking,
  revoking,
  onKick,
  onRevoke,
  onClearExpired,
}: {
  row: MemberRow;
  nowSecs: number;
  kicking?: boolean;
  revoking?: boolean;
  onKick: (nodeId: number) => void;
  onRevoke: (nodeId: number) => void;
  onClearExpired: (nodeId: number) => void;
}) => {
  const action = (() => {
    switch (row.section) {
      case 'joined':
        return (
          <ConfirmActionButton
            label="Remove"
            color="error"
            title="Remove member?"
            body={`Remove node ${row.node_id} from the family? This cannot be undone.`}
            confirmLabel="Remove member"
            disabled={kicking}
            onConfirm={() => onKick(row.node_id)}
            dataTestid={`member-joined-${row.node_id}-kick`}
          />
        );
      case 'pending':
        return row.expired ? (
          <ConfirmActionButton
            label="Clear"
            title="Clear expired invite?"
            body={`Remove the expired invitation for node ${row.node_id}?`}
            confirmLabel="Clear invite"
            disabled={revoking}
            onConfirm={() => onClearExpired(row.node_id)}
            dataTestid={`pending-invite-${row.node_id}-clear`}
          />
        ) : (
          <ConfirmActionButton
            label="Withdraw"
            title="Withdraw invite?"
            body={`Withdraw the pending invitation for node ${row.node_id}?`}
            confirmLabel="Withdraw invite"
            disabled={revoking}
            onConfirm={() => onRevoke(row.node_id)}
            dataTestid={`pending-invite-${row.node_id}-withdraw`}
          />
        );
      default:
        return null;
    }
  })();

  const status = (() => {
    switch (row.section) {
      case 'joined':
        return <StatusChip status="joined" />;
      case 'pending':
        return row.expired ? (
          <StatusChip status="expired" data-testid={`pending-invite-${row.node_id}-expired`} />
        ) : (
          <Stack direction="row" alignItems="center" spacing={1}>
            <StatusChip status="pending" />
            <Typography variant="caption" color="text.secondary">
              {formatExpiry(row.expires_at, nowSecs)}
            </Typography>
          </Stack>
        );
      case 'rejected':
        return <StatusChip status="rejected" />;
      default:
        return <StatusChip status="removed" />;
    }
  })();

  const rowTestId =
    row.section === 'pending' ? `pending-invite-${row.node_id}` : `member-${row.section}-${row.node_id}`;

  return (
    <TableRow hover data-testid={rowTestId}>
      <NodeCell id={row.node_id} />
      <TableCell sx={cellSx}>{status}</TableCell>
      <TableCell align="right" sx={{ ...cellSx, whiteSpace: 'nowrap' }}>
        {action}
      </TableCell>
    </TableRow>
  );
};

/**
 * Single delegations-style table for every family node: one row each for joined
 * members, pending invites and rejected/removed history, with the row's status
 * and the action applicable to that status.
 */
export const FamilyMembersTable = ({
  sections,
  pending,
  nowSecs,
  isLoading,
  isError,
  kicking,
  revoking,
  onKick,
  onRevoke,
  onClearExpired,
  onRefresh,
}: FamilyMembersTableProps) => {
  // Order: current members first, then outstanding invites, then historical records.
  const rows: MemberRow[] = [...sections.joined, ...pending, ...sections.rejected, ...sections.removed];

  return (
    <Stack spacing={2} data-testid="member-list">
      <Stack direction="row" alignItems="center" justifyContent="space-between">
        <Typography variant="subtitle1" fontWeight={600}>
          Members
        </Typography>
        <Button variant="text" size="small" onClick={onRefresh} disabled={isLoading} data-testid="member-list-refresh">
          Refresh
        </Button>
      </Stack>

      {isError ? (
        <Stack spacing={2}>
          <Typography color="error" data-testid="member-list-error">
            Failed to load the member list.
          </Typography>
          <Box>
            <Button variant="outlined" size="small" onClick={onRefresh}>
              Retry
            </Button>
          </Box>
        </Stack>
      ) : (
        <TableContainer
          sx={{
            width: '100%',
            overflowX: 'auto',
            borderRadius: 2,
            border: (t) => `1px solid ${t.palette.divider}`,
          }}
        >
          <Table size="small" sx={{ tableLayout: 'fixed', '& tbody tr:last-child td': { borderBottom: 'none' } }}>
            <TableHead>
              <TableRow>
                <TableCell sx={{ fontWeight: 600, py: 1.25, width: NODE_COL_WIDTH }}>Node</TableCell>
                <TableCell sx={{ fontWeight: 600, py: 1.25, width: STATUS_COL_WIDTH }}>Status</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600, py: 1.25 }}>
                  Actions
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {isLoading && rows.length === 0 && (
                <TableRow>
                  <TableCell colSpan={3} sx={{ py: 2, borderBottom: 'none' }}>
                    <Typography variant="body2" color="text.secondary" data-testid="member-list-loading">
                      Loading members…
                    </Typography>
                  </TableCell>
                </TableRow>
              )}
              {!isLoading && rows.length === 0 && (
                <TableRow>
                  <TableCell colSpan={3} sx={{ py: 2, borderBottom: 'none' }}>
                    <Typography variant="body2" color="text.secondary" data-testid="member-list-empty">
                      No members or invites yet.
                    </Typography>
                  </TableCell>
                </TableRow>
              )}
              {rows.map((row) => (
                <MemberTableRow
                  key={`${row.section}-${row.node_id}`}
                  row={row}
                  nowSecs={nowSecs}
                  kicking={kicking}
                  revoking={revoking}
                  onKick={onKick}
                  onRevoke={onRevoke}
                  onClearExpired={onClearExpired}
                />
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      )}
    </Stack>
  );
};
