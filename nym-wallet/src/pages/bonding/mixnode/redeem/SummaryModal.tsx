import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
  rewards: MajorCurrencyAmount;
  fee?: MajorCurrencyAmount | null;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, rewards, fee }: Props) => (
  <SimpleModal
    open={open}
    onClose={onClose}
    onOk={onConfirm}
    onBack={onCancel}
    header="Redeem rewards"
    subHeader="Claim your rewards"
    okLabel="Redeem rewards"
  >
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Rewards to redeem</Typography>
      <Typography fontWeight={400}>{`${rewards.amount} ${rewards.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Fee for this operation</Typography>
      <Typography fontWeight={400}>{fee ? `${fee.amount} ${fee.denom}` : ''}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Typography fontWeight={400}>Rewards will be transferred to the account you are logged in with</Typography>
  </SimpleModal>
);

export default SummaryModal;
