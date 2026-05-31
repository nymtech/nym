/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { Stack, Typography } from '@mui/material';
import { useSnackbar } from 'notistack';
import { useFamiliesContext, useFamilyById, useFamilyMembership, useOperatorNodeInvites } from 'src/context/families';
import { LeaveFamilyButton, NodeInviteGroup, familyErrorMessage } from 'src/components/Families';
import { NymCard } from 'src/components/NymCard';

const OperatorNodeSection = ({ nodeId }: { nodeId: number }) => {
  const ctx = useFamiliesContext();
  const invites = useOperatorNodeInvites(nodeId);
  const membership = useFamilyMembership(nodeId);
  const { enqueueSnackbar } = useSnackbar();

  const familyId = membership.data?.family_id ?? undefined;
  const family = useFamilyById(familyId);

  const handleAccept = async (fid: number) => {
    try {
      await ctx.acceptFamilyInvitation({ family_id: fid, node_id: nodeId });
      enqueueSnackbar('Invite accepted', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(familyErrorMessage(e), { variant: 'error' });
    }
  };

  const handleReject = async (fid: number) => {
    try {
      await ctx.rejectFamilyInvitation({ family_id: fid, node_id: nodeId });
      enqueueSnackbar('Invite rejected', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(familyErrorMessage(e), { variant: 'error' });
    }
  };

  const handleLeave = async () => {
    try {
      await ctx.leaveFamily({ node_id: nodeId });
      enqueueSnackbar('Left family', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(familyErrorMessage(e), { variant: 'error' });
    }
  };

  return (
    <Stack spacing={2} data-testid={`operator-node-${nodeId}`}>
      {familyId !== undefined && family.data && (
        <NymCard title="Current family" data-testid={`operator-node-${nodeId}-family`}>
          <Stack spacing={2}>
            <Typography variant="body2">
              Node {nodeId} is a member of <strong>{family.data.name}</strong>.
            </Typography>
            <LeaveFamilyButton familyName={family.data.name} isBusy={ctx.isExecuting} onLeave={handleLeave} />
          </Stack>
        </NymCard>
      )}
      <NodeInviteGroup
        nodeId={nodeId}
        invites={invites.data ?? []}
        nowSecs={ctx.nowSecs}
        isBusy={ctx.isExecuting}
        onAccept={handleAccept}
        onReject={handleReject}
      />
    </Stack>
  );
};

/** Operator surface — pending invites per controlled node, plus leave for member nodes. */
export const OperatorInvitesPage = () => {
  const { controlledNodeIds } = useFamiliesContext();

  if (controlledNodeIds.length === 0) {
    return (
      <NymCard title="Node invites" data-testid="operator-invites-empty">
        <Typography variant="body2" color="text.secondary">
          You do not control any bonded nodes, so there are no family invites to show.
        </Typography>
      </NymCard>
    );
  }

  return (
    <Stack spacing={3} data-testid="operator-invites-page">
      {controlledNodeIds.map((nodeId) => (
        <OperatorNodeSection key={nodeId} nodeId={nodeId} />
      ))}
    </Stack>
  );
};
