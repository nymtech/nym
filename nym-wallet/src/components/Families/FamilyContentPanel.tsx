import React from 'react';
import { Box, Stack } from '@mui/material';

/** Delegation-page-style content shell: a single bordered panel for a family tab. */
export const FamilyContentPanel: FCWithChildren<{ 'data-testid'?: string }> = ({ children, 'data-testid': testId }) => (
  <Box
    data-testid={testId}
    sx={{
      p: { xs: 2, md: 3 },
      maxWidth: '100%',
      overflowX: 'hidden',
      borderRadius: 4,
      bgcolor: 'background.paper',
      border: (t) => `1px solid ${t.palette.divider}`,
      boxShadow: (t) => t.palette.nym.nymWallet.shadows.light,
    }}
  >
    <Stack spacing={3}>{children}</Stack>
  </Box>
);
