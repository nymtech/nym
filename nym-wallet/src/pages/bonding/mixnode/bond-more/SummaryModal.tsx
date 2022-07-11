import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
  currentBond: MajorCurrencyAmount;
  addBond: MajorCurrencyAmount;
  fee: MajorCurrencyAmount;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, currentBond, addBond, fee }: Props) => (
  <SimpleModal
    open={open}
    onClose={onClose}
    onOk={onConfirm}
    onBack={onCancel}
    header="Bond mor details"
    okLabel="Confirm"
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
  </SimpleModal>
);

export default SummaryModal;
