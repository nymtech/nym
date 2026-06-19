/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { OperatorInviteView } from 'src/types/families';
import { NymCard } from '../NymCard';
import { ConfirmActionButton } from './ConfirmActionButton';
import { StatusChip } from './StatusChip';
import { formatExpiry, truncateAddress } from './helpers';

export type InviteCardData = OperatorInviteView;

export interface InviteCardProps {
  invite: InviteCardData;
  nowSecs: number;
  isBusy?: boolean;
  onAccept: () => void;
  onReject: () => void;
}

export const InviteCard = ({ invite, nowSecs, isBusy, onAccept, onReject }: InviteCardProps) => (
  <NymCard borderless title={invite.family_name} data-testid={`invite-card-${invite.family_id}`}>
    <Stack spacing={2}>
      <Stack direction="row" spacing={1} alignItems="center" justifyContent="space-between">
        <Typography variant="body2" color="text.secondary">
          Invited by {truncateAddress(invite.owner_address)}
        </Typography>
        {invite.expired ? (
          <StatusChip status="expired" data-testid={`invite-card-${invite.family_id}-expired`} />
        ) : (
          <StatusChip status="active" label={formatExpiry(invite.expires_at, nowSecs)} />
        )}
      </Stack>

      {invite.expired ? (
        <Typography variant="body2" color="text.secondary">
          This invitation has expired and can no longer be accepted.
        </Typography>
      ) : (
        <Box>
          <Stack direction="row" spacing={2}>
            <ConfirmActionButton
              label="Accept"
              variant="contained"
              title="Accept this invite?"
              body="Accepting this invite has on-chain consequences and records your node as a member of this family."
              confirmLabel="Accept invite"
              disabled={isBusy}
              onConfirm={onAccept}
              dataTestid={`invite-card-${invite.family_id}-accept`}
            />
            <ConfirmActionButton
              label="Reject"
              color="error"
              title="Reject this invite?"
              body="Reject this family invitation? It will no longer be shown."
              confirmLabel="Reject invite"
              disabled={isBusy}
              onConfirm={onReject}
              dataTestid={`invite-card-${invite.family_id}-reject`}
            />
          </Stack>
        </Box>
      )}
    </Stack>
  </NymCard>
);
