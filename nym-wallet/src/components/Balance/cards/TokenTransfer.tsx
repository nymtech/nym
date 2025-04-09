import React from 'react';
import { Card, Stack, Button, Skeleton } from '@mui/material';
import { ModalListItem } from 'src/components/Modals/ModalListItem';

export const TokenTransfer = ({
  onTransfer,
  unlockedTokens,
  unlockedRewards,
  unlockedTransferable,
  isLoading,
}: {
  unlockedTokens?: string;
  unlockedRewards?: string;
  unlockedTransferable?: string;
  onTransfer: () => void;
  isLoading?: boolean;
}) => (
  <Card variant="outlined" sx={{ p: 3, height: '100%' }}>
    <Stack justifyContent="space-between" sx={{ height: '100%' }}>
      <Stack gap={1} sx={{ mb: 2 }}>
        <ModalListItem label="Unlocked tokens" value={isLoading ? <Skeleton width={80} /> : unlockedTokens} />
        <ModalListItem label="Unlocked rewards" value={isLoading ? <Skeleton width={80} /> : unlockedRewards} divider />
        <ModalListItem
          fontSize={16}
          label="Transferable tokens"
          value={isLoading ? <Skeleton width={100} /> : unlockedTransferable}
          fontWeight={600}
        />
      </Stack>
      <Button size="large" fullWidth variant="contained" onClick={onTransfer} disableElevation disabled={isLoading}>
        Transfer
      </Button>
    </Stack>
  </Card>
);
