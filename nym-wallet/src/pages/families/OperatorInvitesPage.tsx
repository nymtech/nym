/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { useSnackbar } from 'notistack';
import { useFamiliesContext, useFamilyById, useFamilyMembership, useOperatorNodeInvites } from 'src/context/families';
import { FamilyContentPanel, NodeInviteGroup, familyErrorMessage } from 'src/components/Families';

const OperatorNodeSection = ({ nodeId }: { nodeId: number }) => {
  const ctx = useFamiliesContext();
  const invites = useOperatorNodeInvites(nodeId);
  const membership = useFamilyMembership(nodeId);
  const memberFamily = useFamilyById(membership.data?.family_id ?? undefined);
  const { enqueueSnackbar } = useSnackbar();

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

  const actionBusy = ctx.executingAction === 'accept' || ctx.executingAction === 'reject';

  return (
    <Stack spacing={2} data-testid={`operator-node-${nodeId}`}>
      <Typography variant="subtitle1" fontWeight={600}>
        Node {nodeId}
      </Typography>
      <NodeInviteGroup
        embedded
        nodeId={nodeId}
        invites={invites.data ?? []}
        memberFamilyName={memberFamily.data?.name}
        nowSecs={ctx.nowSecs}
        isBusy={actionBusy}
        onAccept={handleAccept}
        onReject={handleReject}
      />
    </Stack>
  );
};

/** Operator surface: pending invites per controlled node. */
export const OperatorInvitesPage = () => {
  const { controlledNodeIds } = useFamiliesContext();

  if (controlledNodeIds.length === 0) {
    return (
      <FamilyContentPanel data-testid="operator-invites-empty">
        <Typography variant="body2" color="text.secondary">
          You do not control any bonded nodes, so there are no family invites to show.
        </Typography>
      </FamilyContentPanel>
    );
  }

  return (
    <FamilyContentPanel data-testid="operator-invites-page">
      {controlledNodeIds.map((nodeId, index) => (
        <React.Fragment key={nodeId}>
          {index > 0 && <Divider />}
          <OperatorNodeSection nodeId={nodeId} />
        </React.Fragment>
      ))}
    </FamilyContentPanel>
  );
};
