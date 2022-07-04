import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleDialog } from '../../../../components';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  onCancel: () => void;
  rewards: MajorCurrencyAmount;
  fee: MajorCurrencyAmount;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, rewards, fee }: Props) => (
  <SimpleDialog
    open={open}
    onClose={onClose}
    onConfirm={onConfirm}
    onCancel={onCancel}
    title="Compound rewards"
    subTitle="Get more rewards by compounding"
    confirmButton="Compound"
    closeButton
    maxWidth="xs"
    fullWidth
  >
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Operator rewards</Typography>
      <Typography fontWeight={400}>{`${rewards.amount} ${rewards.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Fee for this operation</Typography>
      <Typography fontWeight={400}>{`${fee.amount} ${fee.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Typography fontWeight={400}>Rewards will be added to your bonding pool</Typography>
  </SimpleDialog>
);

export default SummaryModal;
