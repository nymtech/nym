import React from 'react';
import { Box, Chip, Paper, Stack, Tooltip, Typography } from '@mui/material';
import { WrappedDelegationEvent } from '@nymproject/types';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';

export const PendingDelegationCard = ({ item, explorerUrl }: { item: WrappedDelegationEvent; explorerUrl: string }) => (
  <Paper
    variant="outlined"
    sx={{
      p: 2,
      borderRadius: 3,
      bgcolor: (t) => (t.palette.mode === 'dark' ? 'nym.nymWallet.nav.background' : 'nym.nymWallet.background.subtle'),
      borderColor: 'divider',
    }}
  >
    <Stack spacing={1.5} direction={{ xs: 'column', sm: 'row' }} alignItems={{ sm: 'center' }} flexWrap="wrap">
      <Link
        target="_blank"
        href={`${explorerUrl}/nodes/${item.event.mix_id}`}
        text={`${item.node_identity.slice(0, 6)}...${item.node_identity.slice(-6)}`}
        color="text.primary"
        noIcon
      />
      <Typography variant="body2" color="text.secondary">
        {item.event.amount?.amount} {item.event.amount?.denom?.toUpperCase() ?? 'NYM'}
      </Typography>
      <Box sx={{ flex: 1 }} />
      <Tooltip
        title={
          <Box sx={{ textAlign: 'left' }}>
            Your delegation of {item.event.amount?.amount} {item.event.amount?.denom} will take effect when the new
            epoch starts. There is a new epoch every hour.
          </Box>
        }
        arrow
        PopperProps={{
          sx: {
            '& .MuiTooltip-tooltip': { textAlign: 'left' },
          },
        }}
      >
        <Chip label="Pending" size="small" color="primary" variant="outlined" />
      </Tooltip>
    </Stack>
  </Paper>
);
