/* eslint-disable @typescript-eslint/naming-convention */
import React, { useState } from 'react';
import { Box, CircularProgress, Stack, Tab, Tabs, Typography } from '@mui/material';
import { useFamiliesContext, useOwnedFamily, usePendingInviteCountForNodes } from 'src/context/families';
import { InviteNotificationBadge } from 'src/components/Families';
import { PageLayout } from 'src/layouts';
import { CreateFamilyEntry, OwnerManagementPage } from './OwnerManagementPage';
import { OperatorInvitesPage } from './OperatorInvitesPage';

const OwnerTab = () => {
  const { family, isPending } = useOwnedFamily();

  if (isPending) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 8 }} data-testid="family-owner-loading">
        <CircularProgress />
      </Box>
    );
  }

  return family ? <OwnerManagementPage family={family} /> : <CreateFamilyEntry />;
};

/** The Family tab content. Always visible; adapts to ownership and exposes operator invites. */
export const FamilyPage = () => {
  const [tab, setTab] = useState(0);
  const { controlledNodeIds } = useFamiliesContext();
  // Live count of invites awaiting a decision, used to flag the Invites tab.
  const inviteCount = usePendingInviteCountForNodes(controlledNodeIds);

  return (
    <PageLayout>
      <Stack spacing={3} data-testid="family-page">
        <Stack spacing={0.5}>
          <Typography variant="h5" fontWeight={600}>
            Family
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Group your nodes under a family wallet. Owners coordinate membership; operators keep full control of their
            own nodes.
          </Typography>
        </Stack>

        <Tabs
          value={tab}
          onChange={(_e, v) => setTab(v)}
          aria-label="family tabs"
          sx={{
            minHeight: 0,
            borderBottom: (t) => `1px solid ${t.palette.divider}`,
            '& .MuiTab-root': { textTransform: 'none', fontWeight: 600, fontSize: 14, minHeight: 44, px: 0, mr: 3 },
          }}
        >
          <Tab label="My family" data-testid="family-tab-owner" />
          <Tab
            data-testid="family-tab-operator"
            label={
              <InviteNotificationBadge
                badgeContent={inviteCount}
                data-testid="family-invites-badge"
                sx={{ pr: inviteCount > 0 ? 1.5 : 0 }}
              >
                Invites
              </InviteNotificationBadge>
            }
          />
        </Tabs>

        <Box hidden={tab !== 0}>{tab === 0 && <OwnerTab />}</Box>
        <Box hidden={tab !== 1}>{tab === 1 && <OperatorInvitesPage />}</Box>
      </Stack>
    </PageLayout>
  );
};
