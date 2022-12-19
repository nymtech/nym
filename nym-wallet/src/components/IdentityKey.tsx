import React from 'react';
import { Stack, Typography } from '@mui/material';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { splice } from 'src/utils';

export const IdentityKey = ({ identityKey }: { identityKey: string }) => (
  <Stack direction="row">
    <Typography variant="body2" component="span" fontWeight={400} sx={{ mr: 1, color: 'text.primary' }}>
      {splice(6, identityKey)}
    </Typography>
    <CopyToClipboard value={identityKey} sx={{ fontSize: 18 }} />
  </Stack>
);
