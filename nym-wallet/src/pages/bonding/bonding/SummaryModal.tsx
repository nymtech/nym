import React, { useContext, useEffect, useState } from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { getGasFee } from '../../../requests';
import { NodeType } from '../types';
import { AppContext } from '../../../context';
import { SimpleDialog } from '../components';

export interface Props {
  open: boolean;
  onClose?: () => void;
  onSubmit: () => Promise<void>;
  identityKey: string;
  nodeType: NodeType;
  amount: MajorCurrencyAmount;
}

const SummaryModal = ({ open, onClose, onSubmit, identityKey, nodeType, amount }: Props) => {
  const onConfirm = async () => onSubmit();
  const [fee, setFee] = useState<string>('-');
  const { clientDetails } = useContext(AppContext);

  const getFee = async (op: 'BondMixnode' | 'BondGateway') => {
    const res = await getGasFee(op);
    setFee(`${res.amount} ${clientDetails?.denom}`);
  };

  useEffect(() => {
    getFee(nodeType === 'mixnode' ? 'BondMixnode' : 'BondGateway');
  }, [clientDetails, nodeType]);

  return (
    <SimpleDialog
      open={open}
      onClose={onClose}
      onCancel={onClose}
      onConfirm={onConfirm}
      title="Bond details"
      confirmButton="Confirm"
      fullWidth
      maxWidth="xs"
      cancelButton
      closeButton
    >
      <Stack direction="row" justifyContent="space-between">
        <Typography>Identity Key</Typography>
        <Typography>{identityKey}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between">
        <Typography>Amount</Typography>
        <Typography>{`${amount.amount} ${amount.denom}`}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between">
        <Typography>Fee for this operation</Typography>
        <Typography>{fee}</Typography>
      </Stack>
    </SimpleDialog>
  );
};

export default SummaryModal;
