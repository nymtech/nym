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
  /** When set, the node is already in this family, shown instead of the empty-invites message. */
  memberFamilyName?: string;
  /** When true, render inline without an outer NymCard (parent supplies the section heading). */
  embedded?: boolean;
  onAccept: (familyId: number) => void;
  onReject: (familyId: number) => void;
}

const FamilyName = ({ name }: { name: string }) => (
  <Typography component="span" variant="body2" color="primary.main" fontWeight={600}>
    {name}
  </Typography>
);

/** Invitations addressed to a single controlled node (multi-node aware grouping). */
export const NodeInviteGroup = ({
  nodeId,
  invites,
  nowSecs,
  isBusy,
  memberFamilyName,
  embedded,
  onAccept,
  onReject,
}: NodeInviteGroupProps) => {
  const emptyState = memberFamilyName ? (
    <Typography variant="body2" color="text.secondary" data-testid={`node-invite-group-${nodeId}-member`}>
      This node is already a member of <FamilyName name={memberFamilyName} />. <br /> Leave that family in order to join
      another.
    </Typography>
  ) : (
    <Typography variant="body2" color="text.secondary" data-testid={`node-invite-group-${nodeId}-empty`}>
      No invitations for this node.
    </Typography>
  );

  const content =
    invites.length > 0 ? (
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
    ) : (
      emptyState
    );

  if (embedded) {
    return <div data-testid={`node-invite-group-${nodeId}`}>{content}</div>;
  }

  return (
    <NymCard title={`Node ${nodeId}`} data-testid={`node-invite-group-${nodeId}`}>
      {content}
    </NymCard>
  );
};
