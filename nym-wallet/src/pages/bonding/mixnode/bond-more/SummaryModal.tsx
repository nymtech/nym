import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleDialog } from '../../../../components';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  onCancel: () => void;
  currentBond: MajorCurrencyAmount;
  addBond: MajorCurrencyAmount;
  fee: MajorCurrencyAmount;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, currentBond, addBond, fee }: Props) => (
  <SimpleDialog
    open={open}
    onClose={onClose}
    onConfirm={onConfirm}
    onCancel={onCancel}
    title="Bond mor details"
    confirmButton="Confirm"
    closeButton
    cancelButton
    maxWidth="xs"
    fullWidth
  >
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Current bond</Typography>
      <Typography fontWeight={400}>{`${currentBond.amount} ${currentBond.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Additional bond</Typography>
      <Typography fontWeight={400}>{`${addBond.amount} ${addBond.denom}`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Fee for this operation</Typography>
      <Typography fontWeight={400}>{`${fee.amount} ${fee.denom}`}</Typography>
    </Stack>
  </SimpleDialog>
);

export default SummaryModal;
