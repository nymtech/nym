/* eslint-disable @typescript-eslint/naming-convention */
import React, { useState } from 'react';
import { Box, Button, Divider, Stack, Typography } from '@mui/material';
import { SettingsOutlined } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import { useSnackbar } from 'notistack';
import { NodeFamily, PendingMemberRow } from 'src/types/families';
import {
  useFamiliesContext,
  useFamilyConfig,
  useFamilyMemberList,
  useFamilyMembership,
  usePendingInvitationsForFamily,
} from 'src/context/families';
import {
  CreateFamilyForm,
  FamilyContentPanel,
  familyErrorMessage,
  FamilyMembersTable,
  InviteNodeForm,
  InviteWarning,
  inviteWarningFromError,
  MyNodeFamilySection,
} from 'src/components/Families';
import { formatCoin } from 'src/components/Families/helpers';
import { alpha, Theme } from '@mui/material/styles';

export interface OwnerManagementPageProps {
  family: NodeFamily;
}

const StatTile = ({ label, value }: { label: string; value: React.ReactNode }) => (
  <Stack
    spacing={0.5}
    sx={{
      flex: 1,
      minWidth: 120,
      p: 2,
      borderRadius: 2,
      border: (t: Theme) => `1px solid ${t.palette.divider}`,
      bgcolor: (t: Theme) =>
        t.palette.mode === 'dark' ? alpha(t.palette.common.white, 0.04) : alpha(t.palette.common.black, 0.02),
    }}
  >
    <Typography variant="caption" color="text.secondary" sx={{ textTransform: 'uppercase', letterSpacing: 0.5 }}>
      {label}
    </Typography>
    <Typography variant="h6" fontWeight={600}>
      {value}
    </Typography>
  </Stack>
);

const ControlledNodeSections = ({
  onLeave,
  managedFamily,
}: {
  onLeave: (nodeId: number) => void;
  managedFamily?: NodeFamily;
}) => {
  const { controlledNodeIds } = useFamiliesContext();
  if (controlledNodeIds.length === 0) return null;

  return (
    <>
      {controlledNodeIds.map((nodeId) => (
        <MyNodeFamilySection
          key={nodeId}
          nodeId={nodeId}
          managedFamilyId={managedFamily?.id}
          managedFamilyName={managedFamily?.name}
          onLeave={() => onLeave(nodeId)}
        />
      ))}
    </>
  );
};

/** Composed owner management surface (Family Detail), shown when the account owns a family. */
export const OwnerManagementPage = ({ family }: OwnerManagementPageProps) => {
  const navigate = useNavigate();
  const ctx = useFamiliesContext();
  const config = useFamilyConfig();
  const memberList = useFamilyMemberList(family.id);
  const pending = usePendingInvitationsForFamily(family.id);
  const { enqueueSnackbar } = useSnackbar();

  const [inviteWarning, setInviteWarning] = useState<InviteWarning>();
  const [inviteError, setInviteError] = useState<string>();

  const actionBusy = ctx.executingAction !== null;

  const pendingRows: PendingMemberRow[] = (pending.data ?? []).map((d) => ({
    section: 'pending',
    node_id: d.invitation.node_id,
    expires_at: d.invitation.expires_at,
    expired: d.expired,
  }));

  const handleInvite = async (nodeId: number) => {
    setInviteWarning(undefined);
    setInviteError(undefined);
    try {
      await ctx.inviteToFamily({
        node_id: nodeId,
        validity_secs: config.data?.default_invitation_validity_secs,
      });
      await pending.refetch();
      enqueueSnackbar(`Invite sent to node ${nodeId}`, { variant: 'success' });
    } catch (e) {
      const warning = inviteWarningFromError(e);
      if (warning) setInviteWarning(warning);
      else setInviteError(familyErrorMessage(e));
    }
  };

  const handleRevoke = async (nodeId: number) => {
    try {
      await ctx.revokeFamilyInvitation({ node_id: nodeId });
      enqueueSnackbar('Invite withdrawn', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(familyErrorMessage(e), { variant: 'error' });
    }
  };

  const handleKick = async (nodeId: number) => {
    try {
      await ctx.kickFromFamily({ node_id: nodeId });
      enqueueSnackbar(`Removed node ${nodeId}`, { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(familyErrorMessage(e), { variant: 'error' });
    }
  };

  const handleLeave = async (nodeId: number) => {
    try {
      await ctx.leaveFamily({ node_id: nodeId });
      enqueueSnackbar('Left family', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(familyErrorMessage(e), { variant: 'error' });
    }
  };

  return (
    <Stack spacing={3}>
      <ControlledNodeSections onLeave={handleLeave} managedFamily={family} />

      <FamilyContentPanel data-testid="owner-management-page">
        <Stack spacing={1} data-testid="family-summary">
          <Stack direction="row" alignItems="flex-start" justifyContent="space-between" gap={2}>
            <Stack spacing={1} sx={{ minWidth: 0 }}>
              <Typography variant="h6" fontWeight={600}>
                {family.name}
              </Typography>
              {family.description && (
                <Typography variant="body2" color="text.secondary">
                  {family.description}
                </Typography>
              )}
            </Stack>
            <Button
              variant="text"
              color="secondary"
              startIcon={<SettingsOutlined />}
              onClick={() => navigate('/family/settings')}
              data-testid="family-settings-button"
              sx={{ flexShrink: 0 }}
            >
              Family Settings
            </Button>
          </Stack>
          <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} sx={{ pt: 1 }}>
            <StatTile label="Members" value={family.members} />
            <StatTile label="Family ID" value={`#${family.id}`} />
            <StatTile label="Refundable bond" value={formatCoin(family.paid_fee)} />
          </Stack>
        </Stack>

        <Divider />

        <InviteNodeForm
          embedded
          invitationValiditySecs={config.data?.default_invitation_validity_secs}
          isSubmitting={ctx.executingAction === 'invite'}
          isBlocked={actionBusy && ctx.executingAction !== 'invite'}
          warning={inviteWarning}
          errorMessage={inviteError}
          onSubmit={handleInvite}
        />

        <Divider />

        <FamilyMembersTable
          sections={memberList.sections}
          pending={pendingRows}
          nowSecs={ctx.nowSecs}
          isLoading={memberList.isLoading}
          isError={memberList.isError}
          kicking={ctx.executingAction === 'kick'}
          revoking={ctx.executingAction === 'revoke'}
          onKick={handleKick}
          onRevoke={handleRevoke}
          onClearExpired={handleRevoke}
          onRefresh={memberList.refetch}
        />
      </FamilyContentPanel>
    </Stack>
  );
};

/** Create entry point, shown when the account owns no family. */
export const CreateFamilyEntry = () => {
  const ctx = useFamiliesContext();
  const config = useFamilyConfig();
  const { enqueueSnackbar } = useSnackbar();
  const [error, setError] = useState<string>();

  const bondedNodeId = ctx.controlledNodeIds[0];
  const nodeMembership = useFamilyMembership(bondedNodeId);
  const nodeInFamily = bondedNodeId !== undefined && nodeMembership.data?.family_id != null;

  const handleLeave = async (nodeId: number) => {
    try {
      await ctx.leaveFamily({ node_id: nodeId });
      enqueueSnackbar('Left family', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(familyErrorMessage(e), { variant: 'error' });
    }
  };

  const handleCreate = async (name: string, description: string) => {
    setError(undefined);
    if (!config.data || nodeInFamily) return;
    try {
      await ctx.createFamily({ name, description, fee: config.data.create_family_fee });
      enqueueSnackbar('Family created', { variant: 'success' });
    } catch (e) {
      setError(familyErrorMessage(e));
    }
  };

  if (!config.data) {
    return (
      <FamilyContentPanel>
        <Typography color="text.secondary" data-testid="create-family-loading">
          Loading…
        </Typography>
      </FamilyContentPanel>
    );
  }

  // The node belongs to another wallet's family: MyNodeFamilySection brings its own panel.
  if (nodeInFamily) {
    return <ControlledNodeSections onLeave={handleLeave} />;
  }

  return (
    <FamilyContentPanel>
      <CreateFamilyForm
        embedded
        fee={config.data.create_family_fee}
        nameLimit={config.data.family_name_length_limit}
        descriptionLimit={config.data.family_description_length_limit}
        isSubmitting={ctx.executingAction === 'create'}
        isBlocked={ctx.executingAction !== null && ctx.executingAction !== 'create'}
        errorMessage={error}
        onSubmit={handleCreate}
      />
    </FamilyContentPanel>
  );
};
