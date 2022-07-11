import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
  currentPm: number;
  newPm: number;
  fee?: MajorCurrencyAmount | null;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, currentPm, newPm, fee }: Props) => (
  <SimpleModal
    open={open}
    onClose={onClose}
    onOk={onConfirm}
    onBack={onCancel}
    header="Profit margin change"
    subHeader="System Variables"
    okLabel="Confirm"
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
      <Typography fontWeight={400}>{fee ? `${fee?.amount} ${fee?.denom}` : ''}</Typography>
    </Stack>
  </SimpleModal>
);

export default SummaryModal;
