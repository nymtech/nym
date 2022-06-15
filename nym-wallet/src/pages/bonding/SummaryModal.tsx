import React, { useContext, useEffect, useState } from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleModal } from '../../components/Modals/SimpleModal';
import { getGasFee } from '../../requests';
import { NodeType } from './types';
import { AppContext } from '../../context';

export interface Props {
  open: boolean;
  onClose?: () => void;
  onSubmit: () => Promise<void>;
  header: string;
  buttonText: string;
  identityKey: string;
  nodeType: NodeType;
  amount: MajorCurrencyAmount;
}

export const SummaryModal = ({ open, onClose, onSubmit, header, buttonText, identityKey, nodeType, amount }: Props) => {
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
    <SimpleModal open={open} onClose={onClose} onOk={onConfirm} header={header} okLabel={buttonText}>
      <Stack direction="row" justifyContent="space-between" mt={3}>
        <Typography fontWeight={600}>Identity Key</Typography>
        <Typography fontWeight={600}>{identityKey}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between" mt={3}>
        <Typography fontWeight={600}>Amount</Typography>
        <Typography fontWeight={600}>{`${amount.amount} ${amount.denom}`}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between" mt={3}>
        <Typography fontWeight={600}>Fee for this operation</Typography>
        <Typography fontWeight={600}>{fee}</Typography>
      </Stack>
    </SimpleModal>
  );
};
