/* eslint-disable @typescript-eslint/naming-convention */
import React, { useEffect, useState } from 'react';
import { Close, SettingsOutlined } from '@mui/icons-material';
import { Box, CircularProgress, Divider, IconButton, Stack, Typography } from '@mui/material';
import { useNavigate } from 'react-router-dom';
import { useSnackbar } from 'notistack';
import { DeleteFamilyButton, EditFamilyForm, FamilyContentPanel, familyErrorMessage } from 'src/components/Families';
import { useFamiliesContext, useFamilyConfig, useOwnedFamily } from 'src/context/families';
import { PageLayout } from 'src/layouts';

/** Family owner settings: edit details and dissolve the family. */
export const FamilySettingsPage = () => {
  const navigate = useNavigate();
  const ctx = useFamiliesContext();
  const { family, isPending: familyLoading } = useOwnedFamily();
  const config = useFamilyConfig();
  const { enqueueSnackbar } = useSnackbar();

  const [editError, setEditError] = useState<string>();
  const [deleteError, setDeleteError] = useState<string>();

  const nameLimit = config.data?.family_name_length_limit ?? 30;
  const descLimit = config.data?.family_description_length_limit ?? 120;
  const actionBusy = ctx.executingAction !== null;

  useEffect(() => {
    if (!familyLoading && !family) {
      navigate('/family');
    }
  }, [familyLoading, family, navigate]);

  const handleEdit = async (updatedName: string | null, updatedDescription: string | null) => {
    setEditError(undefined);
    try {
      await ctx.updateFamily({ updated_name: updatedName, updated_description: updatedDescription });
      enqueueSnackbar('Family updated', { variant: 'success' });
    } catch (e) {
      setEditError(familyErrorMessage(e));
    }
  };

  const handleDelete = async () => {
    setDeleteError(undefined);
    try {
      await ctx.disbandFamily();
      enqueueSnackbar('Family dissolved', { variant: 'success' });
      navigate('/family');
    } catch (e) {
      setDeleteError(familyErrorMessage(e));
    }
  };

  if (familyLoading || !config.data) {
    return (
      <PageLayout>
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 8 }} data-testid="family-settings-loading">
          <CircularProgress />
        </Box>
      </PageLayout>
    );
  }

  if (!family) return null;

  return (
    <PageLayout>
      <Stack spacing={3} data-testid="family-settings-page">
        <Stack direction="row" alignItems="center" justifyContent="space-between">
          <Stack direction="row" alignItems="center" spacing={1}>
            <SettingsOutlined sx={{ fontSize: 22, color: 'primary.main' }} />
            <Typography variant="h5" fontWeight={600}>
              Family settings
            </Typography>
          </Stack>
          <IconButton
            aria-label="Close family settings"
            onClick={() => navigate('/family')}
            data-testid="family-settings-close"
          >
            <Close />
          </IconButton>
        </Stack>

        <FamilyContentPanel>
          <EditFamilyForm
            embedded
            initialName={family.name}
            initialDescription={family.description}
            nameLimit={nameLimit}
            descriptionLimit={descLimit}
            isSubmitting={ctx.executingAction === 'update'}
            isBlocked={actionBusy && ctx.executingAction !== 'update'}
            errorMessage={editError}
            onSubmit={handleEdit}
          />

          <Divider />

          <Stack spacing={2} data-testid="dissolve-family-card">
            <Typography variant="subtitle1" fontWeight={600} color="error.main">
              Dissolve family
            </Typography>
            <DeleteFamilyButton
              memberCount={family.members}
              isBusy={ctx.executingAction === 'disband'}
              errorMessage={deleteError}
              onDelete={handleDelete}
            />
          </Stack>
        </FamilyContentPanel>
      </Stack>
    </PageLayout>
  );
};
