import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { DecCoin } from '@nymproject/types';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
  rewards: DecCoin;
  fee?: DecCoin | null;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, rewards, fee }: Props) => (
  <SimpleModal
    open={open}
    onClose={onClose}
    onOk={onConfirm}
    onBack={onCancel}
    header="Compound rewards"
    subHeader="Get more rewards by compounding"
    okLabel="Compound"
  >
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Operator rewards</Typography>
      <Typography fontWeight={400}>{`${rewards.amount} ${rewards.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Fee for this operation</Typography>
      <Typography fontWeight={400}>{fee ? `${fee?.amount} ${fee?.denom}` : ''}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Typography fontWeight={400}>Rewards will be added to your bonding pool</Typography>
  </SimpleModal>
);

export default SummaryModal;
