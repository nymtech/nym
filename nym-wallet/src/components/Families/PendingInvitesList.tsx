/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { Chip, Stack, Table, TableBody, TableCell, TableHead, TableRow, Typography } from '@mui/material';
import { PendingMemberRow } from 'src/types/families';
import { NymCard } from '../NymCard';
import { ConfirmActionButton } from './ConfirmActionButton';
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
            <TableCell>Node</TableCell>
            <TableCell>Expiry</TableCell>
            <TableCell align="right">Action</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {invites.map((inv) => (
            <TableRow key={inv.node_id} data-testid={`pending-invite-${inv.node_id}`}>
              <TableCell>{inv.node_id}</TableCell>
              <TableCell>
                <Stack direction="row" spacing={1} alignItems="center">
                  {inv.expired ? (
                    <Chip
                      size="small"
                      color="default"
                      label="Expired"
                      data-testid={`pending-invite-${inv.node_id}-expired`}
                    />
                  ) : (
                    <Typography variant="body2">{formatExpiry(inv.expires_at, nowSecs)}</Typography>
                  )}
                </Stack>
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
