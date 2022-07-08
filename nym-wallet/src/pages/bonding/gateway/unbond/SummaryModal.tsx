import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleDialog } from '../../../../components';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  onCancel: () => void;
  bond: MajorCurrencyAmount;
  rewards?: MajorCurrencyAmount;
  fee: MajorCurrencyAmount;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, bond, rewards, fee }: Props) => (
  <SimpleDialog
    open={open}
    onClose={onClose}
    onConfirm={onConfirm}
    onCancel={onCancel}
    title="Unbond"
    subTitle="Unbond and remove your node from the mixnet"
    confirmButton="Unbond"
    closeButton
    maxWidth="xs"
    fullWidth
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
      <Typography fontWeight={400}>{`${fee.amount} ${fee.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Typography fontWeight={400}>Tokens will be transferred to account you are logged in with now</Typography>
  </SimpleDialog>
);

export default SummaryModal;
