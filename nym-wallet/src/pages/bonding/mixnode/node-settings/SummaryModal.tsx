import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleDialog } from '../../components';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  onCancel: () => void;
  currentPm: number;
  newPm: number;
  fee: MajorCurrencyAmount;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, currentPm, newPm, fee }: Props) => (
  <SimpleDialog
    open={open}
    onClose={onClose}
    onConfirm={onConfirm}
    onCancel={onCancel}
    title="Profit margin change"
    subTitle="System Variables"
    confirmButton="Confirm"
    closeButton
    cancelButton
    maxWidth="xs"
    fullWidth
  >
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Current profit margin</Typography>
      <Typography fontWeight={400}>{`${currentPm}%`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>New profit margin</Typography>
      <Typography fontWeight={400}>{`${newPm}%`}</Typography>
    </Stack>
    <Divider sx={{ my: 1 }} />
    <Stack direction="row" justifyContent="space-between">
      <Typography fontWeight={400}>Fee for this operation</Typography>
      <Typography fontWeight={400}>{`${fee.amount} ${fee.denom}`}</Typography>
    </Stack>
  </SimpleDialog>
);

export default SummaryModal;
