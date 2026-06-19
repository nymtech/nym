/* eslint-disable @typescript-eslint/naming-convention */
import React, { useState } from 'react';
import { alpha, Theme, useTheme } from '@mui/material/styles';
import { Box, Grid, Stack, Typography } from '@mui/material';
import { useSnackbar } from 'notistack';
import { NodeFamily, PendingMemberRow } from 'src/types/families';
import {
  useFamiliesContext,
  useFamilyConfig,
  useFamilyMemberList,
  usePendingInvitationsForFamily,
} from 'src/context/families';
import {
  CreateFamilyForm,
  DeleteFamilyButton,
  EditFamilyForm,
  familyErrorMessage,
  InviteNodeForm,
  InviteWarning,
  inviteWarningFromError,
  MemberList,
  PendingInvitesList,
} from 'src/components/Families';
import { NymCard } from 'src/components/NymCard';
import { formatCoin } from 'src/components/Families/helpers';

export interface OwnerManagementPageProps {
  family: NodeFamily;
}

const StatTile = ({ label, value }: { label: string; value: React.ReactNode }) => (
  <Box
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
    <Typography variant="h6" fontWeight={600} sx={{ mt: 0.5 }}>
      {value}
    </Typography>
  </Box>
);

/** Composed owner management surface (Family Detail) — shown when the account owns a family. */
export const OwnerManagementPage = ({ family }: OwnerManagementPageProps) => {
  const theme = useTheme();
  const ctx = useFamiliesContext();
  const config = useFamilyConfig();
  const memberList = useFamilyMemberList(family.id);
  const pending = usePendingInvitationsForFamily(family.id);
  const { enqueueSnackbar } = useSnackbar();

  const [editError, setEditError] = useState<string>();
  const [inviteWarning, setInviteWarning] = useState<InviteWarning>();
  const [inviteError, setInviteError] = useState<string>();
  const [deleteError, setDeleteError] = useState<string>();

  const nameLimit = config.data?.family_name_length_limit ?? 30;
  const descLimit = config.data?.family_description_length_limit ?? 120;

  const pendingRows: PendingMemberRow[] = (pending.data ?? []).map((d) => ({
    section: 'pending',
    node_id: d.invitation.node_id,
    expires_at: d.invitation.expires_at,
    expired: d.expired,
  }));

  const handleEdit = async (updatedName: string | null, updatedDescription: string | null) => {
    setEditError(undefined);
    try {
      await ctx.updateFamily({ updated_name: updatedName, updated_description: updatedDescription });
      enqueueSnackbar('Family updated', { variant: 'success' });
    } catch (e) {
      setEditError(familyErrorMessage(e));
    }
  };

  const handleInvite = async (nodeId: number) => {
    setInviteWarning(undefined);
    setInviteError(undefined);
    try {
      await ctx.inviteToFamily({ node_id: nodeId });
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

  const handleDelete = async () => {
    setDeleteError(undefined);
    try {
      await ctx.disbandFamily();
      enqueueSnackbar('Family dissolved', { variant: 'success' });
    } catch (e) {
      setDeleteError(familyErrorMessage(e));
    }
  };

  return (
    <Stack spacing={3} data-testid="owner-management-page">
      <NymCard title={family.name} subheader={family.description} data-testid="family-summary">
        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
          <StatTile label="Members" value={family.members} />
          <StatTile label="Family ID" value={`#${family.id}`} />
          <StatTile label="Refundable bond" value={formatCoin(family.paid_fee)} />
        </Stack>
      </NymCard>

      <Grid container spacing={3}>
        <Grid item xs={12} md={6}>
          <EditFamilyForm
            initialName={family.name}
            initialDescription={family.description}
            nameLimit={nameLimit}
            descriptionLimit={descLimit}
            isSubmitting={ctx.isExecuting}
            errorMessage={editError}
            onSubmit={handleEdit}
          />
        </Grid>
        <Grid item xs={12} md={6}>
          <InviteNodeForm
            isSubmitting={ctx.isExecuting}
            warning={inviteWarning}
            errorMessage={inviteError}
            onSubmit={handleInvite}
          />
        </Grid>
      </Grid>

      <PendingInvitesList
        invites={pendingRows}
        nowSecs={ctx.nowSecs}
        isBusy={ctx.isExecuting}
        onRevoke={handleRevoke}
        onClearExpired={handleRevoke}
      />

      <MemberList
        sections={memberList.sections}
        nowSecs={ctx.nowSecs}
        isLoading={memberList.isLoading}
        isError={memberList.isError}
        isBusy={ctx.isExecuting}
        onKick={handleKick}
        onRefresh={memberList.refetch}
      />

      <NymCard
        title="Dissolve family"
        data-testid="dissolve-family-card"
        sx={{ borderColor: alpha(theme.palette.error.main, 0.4) }}
      >
        <DeleteFamilyButton
          memberCount={family.members}
          isBusy={ctx.isExecuting}
          errorMessage={deleteError}
          onDelete={handleDelete}
        />
      </NymCard>
    </Stack>
  );
};

/** Create entry point — shown when the account owns no family. */
export const CreateFamilyEntry = () => {
  const ctx = useFamiliesContext();
  const config = useFamilyConfig();
  const { enqueueSnackbar } = useSnackbar();
  const [error, setError] = useState<string>();

  const handleCreate = async (name: string, description: string) => {
    setError(undefined);
    if (!config.data) return;
    try {
      await ctx.createFamily({ name, description, fee: config.data.create_family_fee });
      enqueueSnackbar('Family created', { variant: 'success' });
    } catch (e) {
      setError(familyErrorMessage(e));
    }
  };

  if (!config.data) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 8 }} data-testid="create-family-loading">
        <Typography color="text.secondary">Loading…</Typography>
      </Box>
    );
  }

  return (
    <Box sx={{ maxWidth: 560, mx: 'auto', width: '100%' }}>
      <CreateFamilyForm
        fee={config.data.create_family_fee}
        nameLimit={config.data.family_name_length_limit}
        descriptionLimit={config.data.family_description_length_limit}
        isSubmitting={ctx.isExecuting}
        errorMessage={error}
        onSubmit={handleCreate}
      />
    </Box>
  );
};
