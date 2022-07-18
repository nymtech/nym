import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { SimpleModal } from '../../../components/Modals/SimpleModal';
import { DecCoin } from '@nymproject/types';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
  bond: DecCoin;
  rewards?: DecCoin;
  fee?: DecCoin | null;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, bond, rewards, fee }: Props) => (
  <SimpleModal
    open={open}
    onClose={onClose}
    onOk={onConfirm}
    onBack={onCancel}
    header="Unbond"
    subHeader="Unbond and remove your node from the mixnet"
    okLabel="Unbond"
  >
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Amount to unbond</Typography>
      <Typography fontWeight={400}>{`${bond.amount} ${bond.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    {rewards?.amount && (
      <>
        <Stack direction="row" justifyContent="space-between">
          <Typography fontWeight={400}>Operator rewards</Typography>
          <Typography fontWeight={400}>{`${rewards.amount} ${rewards.denom}`}</Typography>
        </Stack>
        <Divider sx={{ my: 1 }} />
      </>
    )}
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Fee for this operation</Typography>
      <Typography fontWeight={400}>{fee ? `${fee.amount} ${fee.denom}` : ''}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Typography fontWeight={400}>Tokens will be transferred to account you are logged in with now</Typography>
  </SimpleModal>
);

export default SummaryModal;
