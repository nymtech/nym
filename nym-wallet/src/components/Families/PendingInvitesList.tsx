/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { Table, TableBody, TableCell, TableHead, TableRow, Typography } from '@mui/material';
import { Theme } from '@mui/material/styles';
import { PendingMemberRow } from 'src/types/families';
import { NymCard } from '../NymCard';
import { ConfirmActionButton } from './ConfirmActionButton';
import { StatusChip } from './StatusChip';
import { formatExpiry } from './helpers';

export interface PendingInvitesListProps {
  invites: PendingMemberRow[];
  nowSecs: number;
  isBusy?: boolean;
  /** Withdraw an active (not-yet-expired) invite. */
  onRevoke: (nodeId: number) => void;
  /** Dismiss/clear an expired invite. */
  onClearExpired: (nodeId: number) => void;
}

const headCellSx = (t: Theme) => ({
  color: t.palette.text.secondary,
  fontWeight: 600,
  borderBottom: `1px solid ${t.palette.divider}`,
});
const rowSx = (t: Theme) => ({ '& td': { borderBottom: `1px solid ${t.palette.divider}`, py: 1.25 } });

export const PendingInvitesList = ({ invites, nowSecs, isBusy, onRevoke, onClearExpired }: PendingInvitesListProps) => (
  <NymCard title="Pending invites" data-testid="pending-invites-list">
    {invites.length === 0 ? (
      <Typography variant="body2" color="text.secondary" data-testid="pending-invites-empty">
        No pending invites.
      </Typography>
    ) : (
      <Table size="small">
        <TableHead>
          <TableRow>
            <TableCell sx={headCellSx}>Node</TableCell>
            <TableCell sx={headCellSx}>Status</TableCell>
            <TableCell sx={headCellSx} align="right">
              Action
            </TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {invites.map((inv) => (
            <TableRow key={inv.node_id} data-testid={`pending-invite-${inv.node_id}`} sx={rowSx}>
              <TableCell>
                <Typography variant="body2" sx={{ fontFamily: 'monospace' }}>
                  Node {inv.node_id}
                </Typography>
              </TableCell>
              <TableCell>
                {inv.expired ? (
                  <StatusChip status="expired" data-testid={`pending-invite-${inv.node_id}-expired`} />
                ) : (
                  <StatusChip status="pending" label={formatExpiry(inv.expires_at, nowSecs)} />
                )}
              </TableCell>
              <TableCell align="right">
                {inv.expired ? (
                  <ConfirmActionButton
                    label="Clear"
                    title="Clear expired invite?"
                    body={`Remove the expired invitation for node ${inv.node_id}?`}
                    confirmLabel="Clear invite"
                    disabled={isBusy}
                    onConfirm={() => onClearExpired(inv.node_id)}
                    dataTestid={`pending-invite-${inv.node_id}-clear`}
                  />
                ) : (
                  <ConfirmActionButton
                    label="Withdraw"
                    title="Withdraw invite?"
                    body={`Withdraw the pending invitation for node ${inv.node_id}?`}
                    confirmLabel="Withdraw invite"
                    disabled={isBusy}
                    onConfirm={() => onRevoke(inv.node_id)}
                    dataTestid={`pending-invite-${inv.node_id}-withdraw`}
                  />
                )}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    )}
  </NymCard>
);
