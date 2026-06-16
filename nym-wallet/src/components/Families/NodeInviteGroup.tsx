/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { Stack, Typography } from '@mui/material';
import { NymCard } from '../NymCard';
import { InviteCard, InviteCardData } from './InviteCard';

export interface NodeInviteGroupProps {
  nodeId: number;
  invites: InviteCardData[];
  nowSecs: number;
  isBusy?: boolean;
  onAccept: (familyId: number) => void;
  onReject: (familyId: number) => void;
}

/** Invitations addressed to a single controlled node (multi-node aware grouping). */
export const NodeInviteGroup = ({ nodeId, invites, nowSecs, isBusy, onAccept, onReject }: NodeInviteGroupProps) => (
  <NymCard title={`Node ${nodeId}`} data-testid={`node-invite-group-${nodeId}`}>
    {invites.length === 0 ? (
      <Typography variant="body2" color="text.secondary" data-testid={`node-invite-group-${nodeId}-empty`}>
        No invitations for this node.
      </Typography>
    ) : (
      <Stack spacing={2}>
        {invites.map((invite) => (
          <InviteCard
            key={invite.family_id}
            invite={invite}
            nowSecs={nowSecs}
            isBusy={isBusy}
            onAccept={() => onAccept(invite.family_id)}
            onReject={() => onReject(invite.family_id)}
          />
        ))}
      </Stack>
    )}
  </NymCard>
);
