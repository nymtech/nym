import React, { useEffect } from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { GatewayAmount, GatewayData, MixnodeAmount, MixnodeData, NodeData } from '../types';
import { SimpleModal } from '../../../components/Modals/SimpleModal';
import { useBondingContext } from '../../../context';

export interface Props {
  open: boolean;
  onClose: () => void;
  onCancel: () => void;
  onSubmit: () => Promise<void>;
  onError: (message: string) => void;
  node: NodeData;
  amount: MixnodeAmount | GatewayAmount;
}

const SummaryModal = ({ open, onClose, onSubmit, node, amount, onCancel }: Props) => {
  const onConfirm = async () => onSubmit();

  return (
    <SimpleModal
      open={open}
      onClose={() => {
        onClose();
      }}
      onBack={() => {
        onCancel();
      }}
      onOk={onConfirm}
      header="Bond details"
      okLabel="Confirm"
    >
      <Stack direction="row" justifyContent="space-between">
        <Typography>Identity Key</Typography>
        <Typography>{node.identityKey}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between">
        <Typography>Amount</Typography>
        <Typography>{`${amount.amount.amount} ${amount.amount.denom}`}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
    </SimpleModal>
  );
};

export default SummaryModal;
