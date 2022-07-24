import * as React from 'react';
import { Typography } from '@mui/material';
import { useEffect, useState } from 'react';
import { TBondedGateway, TBondedMixnode } from 'src/context';
import { NodeType } from 'src/pages/bonding/types';
import { useGetFee } from 'src/hooks/useGetFee';
import { isGateway, isMixnode } from 'src/types';
import { ModalFee } from '../../Modals/ModalFee';
import { ModalListItem } from '../../Modals/ModalListItem';
import { SimpleModal } from '../../Modals/SimpleModal';
import {
  simulateUnbondGateway,
  simulateUnbondMixnode,
  simulateVestingUnbondGateway,
  simulateVestingUnbondMixnode,
} from '../../../requests';

interface Props {
  node: TBondedMixnode | TBondedGateway;
  onConfirm: () => Promise<void>;
  onClose: () => void;
  onError: (e: string) => void;
}

export const UnbondModal = ({ node, onConfirm, onClose, onError }: Props) => {
  const { fee, isFeeLoading, getFee, feeError } = useGetFee();

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  useEffect(() => {
    if (isMixnode(node) && !node.proxy) {
      getFee(simulateUnbondMixnode, {});
    }

    if (isMixnode(node) && node.proxy) {
      getFee(simulateVestingUnbondMixnode, {});
    }

    if (isGateway(node) && !node.proxy) {
      getFee(simulateVestingUnbondGateway, {});
    }

    if (isGateway(node) && node.proxy) {
      getFee(simulateUnbondGateway, {});
    }
  }, [node]);

  return (
    <SimpleModal
      open
      header="Unbond"
      subHeader="Unbond and remove your node from the mixnet"
      okLabel="Unbond"
      onOk={onConfirm}
      onClose={onClose}
    >
      <ModalListItem label="Amount to unbond" value={`${node.bond.amount} ${node.bond.denom.toUpperCase()}`} divider />
      <ModalListItem label="Operator rewards" value={`${node.bond.amount} ${node.bond.denom.toUpperCase()}`} divider />
      <ModalFee isLoading={isFeeLoading} fee={fee} divider />
      <Typography fontSize="small">Tokens will be transferred to the account you are logged in with now</Typography>
    </SimpleModal>
  );
};
