/* eslint-disable @typescript-eslint/naming-convention */
import React, { useState } from 'react';
import { Box, CircularProgress, Stack, Tab, Tabs, Typography } from '@mui/material';
import { useFamiliesContext, useFamilyByOwner } from 'src/context/families';
import { CreateFamilyEntry, OwnerManagementPage } from './OwnerManagementPage';
import { OperatorInvitesPage } from './OperatorInvitesPage';

const OwnerTab = () => {
  const familyByOwner = useFamilyByOwner();

  if (familyByOwner.isPending) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }} data-testid="family-owner-loading">
        <CircularProgress />
      </Box>
    );
  }

  return familyByOwner.data ? <OwnerManagementPage family={familyByOwner.data} /> : <CreateFamilyEntry />;
};

/** The Family tab content — always visible; adapts to ownership and exposes operator invites. */
export const FamilyPage = () => {
  const [tab, setTab] = useState(0);
  // touch the context so the tab is meaningful even before reads resolve
  useFamiliesContext();

  return (
    <Stack spacing={3} sx={{ p: 4 }} data-testid="family-page">
      <Typography variant="h5">Family</Typography>
      <Typography variant="body2" color="text.secondary">
        Coordinate your nodes under a family wallet. Operators stay sovereign; owners delegate, never seize.
      </Typography>

      <Tabs value={tab} onChange={(_e, v) => setTab(v)} aria-label="family tabs">
        <Tab label="My family" data-testid="family-tab-owner" />
        <Tab label="Node invites" data-testid="family-tab-operator" />
      </Tabs>

      <Box hidden={tab !== 0}>{tab === 0 && <OwnerTab />}</Box>
      <Box hidden={tab !== 1}>{tab === 1 && <OperatorInvitesPage />}</Box>
    </Stack>
  );
};
