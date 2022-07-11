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

const SummaryModal = ({ open, onClose, onSubmit, node, amount, onCancel, onError }: Props) => {
  const { fee, getFee, resetFeeState, feeError, feeLoading } = useBondingContext();

  useEffect(() => {
    if (feeError) onError(feeError);
  }, [feeError]);

  const fetchFee = async () => {
    const { signature, host, version, mixPort, identityKey, sphinxKey } = node;
    try {
      if (node.nodeType === 'mixnode') {
        await getFee(amount.tokenPool === 'locked' ? 'bondMixnodeWithVesting' : 'bondMixnode', {
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
        await getFee(amount.tokenPool === 'locked' ? 'bondGatewayWithVesting' : 'bondGateway', {
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
    <SimpleModal
      open={open}
      onClose={() => {
        resetFeeState();
        onClose();
      }}
      onBack={() => {
        resetFeeState();
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
      <Stack direction="row" justifyContent="space-between">
        <Typography>Fee for this operation</Typography>
        {feeLoading ? (
          <Typography>loading</Typography>
        ) : (
          <Typography>{fee ? `${fee.amount?.amount} ${fee.amount?.denom}` : ''}</Typography>
        )}
      </Stack>
    </SimpleModal>
  );
};

export default SummaryModal;
