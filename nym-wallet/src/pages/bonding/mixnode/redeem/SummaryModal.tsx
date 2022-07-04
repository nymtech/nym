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
    title="Redeem rewards"
    subTitle="Claim your rewards"
    confirmButton="Redeem rewards"
    closeButton
    maxWidth="xs"
    fullWidth
  >
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Rewards to redeem</Typography>
      <Typography fontWeight={400}>{`${rewards.amount} ${rewards.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Fee for this operation</Typography>
      <Typography fontWeight={400}>{`${fee.amount} ${fee.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Typography fontWeight={400}>Rewards will be transferred to the account you are logged in with</Typography>
  </SimpleDialog>
);

export default SummaryModal;
