import React from 'react';
import { Card, Stack, Button } from '@mui/material';
import { ModalListItem } from 'src/components/Modals/ModalListItem';

export const TokenTransfer = ({
  onTransfer,
  unlockedTokens,
  unlockedRewards,
  unlockedTransferable,
}: {
  userBalance?: string;
  unlockedTokens?: string;
  unlockedRewards?: string;
  unlockedTransferable?: string;
  onTransfer: () => void;
}) => {
  return (
    <Card variant="outlined" sx={{ p: 3, height: '100%' }}>
      <Stack gap={1} sx={{ mb: 2 }}>
        <ModalListItem label="Unlocked tokens" value={unlockedTokens} />
        <ModalListItem label="Unlocked rewards" value={unlockedRewards} divider />
        <ModalListItem label="Unlocked transferabled tokens" value={unlockedTransferable} fontWeight={600} />
      </Stack>
      <Button size="large" fullWidth variant="contained" onClick={onTransfer} disableElevation>
        Transfer
      </Button>
    </Card>
  );
};
