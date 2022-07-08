import React, { useEffect } from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import {
  simulateBondGateway,
  simulateBondMixnode,
  simulateVestingBondGateway,
  simulateVestingBondMixnode,
} from '../../../requests';
import { GatewayAmount, GatewayData, MixnodeAmount, MixnodeData, NodeData } from '../types';
import { SimpleDialog } from '../../../components';
import { useGetFee } from '../../../hooks/useGetFee';

export interface Props {
  open: boolean;
  onClose: () => void;
  onCancel: () => void;
  onSubmit: () => Promise<void>;
  onError: (message: string) => void;
  node: NodeData;
  amount: MixnodeAmount | GatewayAmount;
}

const SummaryModal = ({ open, onClose, onSubmit, node, amount, onCancel, onError }: Props) => {
  const { fee, getFee, resetFeeState, feeError, isFeeLoading } = useGetFee();

  useEffect(() => {
    if (feeError) onError(feeError);
  }, [feeError]);

  const fetchFee = async () => {
    const { signature, host, version, mixPort, identityKey, sphinxKey } = node;
    try {
      if (node.nodeType === 'mixnode') {
        await getFee(amount.tokenPool === 'locked' ? simulateVestingBondMixnode : simulateBondMixnode, {
          ownerSignature: signature,
          mixnode: {
            identity_key: identityKey,
            sphinx_key: sphinxKey,
            host,
            version,
            mix_port: mixPort,
            profit_margin_percent: (amount as MixnodeAmount).profitMargin,
            verloc_port: (node as NodeData<MixnodeData>).verlocPort,
            http_api_port: (node as NodeData<MixnodeData>).httpApiPort,
          },
          pledge: amount.amount,
        });
      } else {
        await getFee(amount.tokenPool === 'locked' ? simulateVestingBondGateway : simulateBondGateway, {
          ownerSignature: signature,
          gateway: {
            identity_key: identityKey,
            sphinx_key: sphinxKey,
            host,
            version,
            mix_port: mixPort,
            location: (node as NodeData<GatewayData>).location,
            clients_port: (node as NodeData<GatewayData>).clientsPort,
          },
          pledge: amount.amount,
        });
      }
    } catch (e) {
      onError(e as string);
    }
  };

  useEffect(() => {
    fetchFee();
  }, [node, amount]);

  const onConfirm = async () => onSubmit();

  return (
    <SimpleDialog
      open={open}
      onClose={() => {
        resetFeeState();
        onClose();
      }}
      onCancel={() => {
        resetFeeState();
        onCancel();
      }}
      onConfirm={onConfirm}
      title="Bond details"
      confirmButton="Confirm"
      maxWidth="xs"
      fullWidth
      cancelButton
      closeButton
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
      <Stack direction="row" justifyContent="space-between">
        <Typography>Fee for this operation</Typography>
        {isFeeLoading ? (
          <Typography>loading</Typography>
        ) : (
          <Typography>{fee ? `${fee.amount?.amount} ${fee.amount?.denom}` : ''}</Typography>
        )}
      </Stack>
    </SimpleDialog>
  );
};

export default SummaryModal;
