import * as React from 'react';
import { useContext, useEffect, useState } from 'react';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import { ErrorOutline } from '@mui/icons-material';
import { AppContext, TBondedGateway, TBondedMixnode, urls, useBondingContext } from 'src/context';
import SummaryModal from './SummaryModal';
import { ConfirmationModal } from '../../../components';
import { LoadingModal } from '../../../components/Modals/LoadingModal';
import { NodeType } from '../types';
import { useGetFee } from 'src/hooks/useGetFee';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { Network } from 'src/types';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { ModalFee } from 'src/components/Modals/ModalFee';

interface Props {
  node: TBondedMixnode | TBondedGateway;
  show: boolean;
  network: Network;
  onConfirm: () => void;
  onClose: () => void;
}

const Unbond = ({ node, show, onConfirm, onClose }: Props) => {
  const [txHash, setTxHash] = useState<string>();
  const [nodeType, setNodeType] = useState<NodeType>('mixnode');

  const { fee, isFeeLoading, getFee, resetFeeState } = useGetFee();

  const handleGetFee = async () => {};

  useEffect(() => {
    if ('profitMargin' in node) {
      setNodeType('mixnode');
    } else {
      setNodeType('gateway');
    }
  }, [node]);

  return (
    <SimpleModal
      open
      header="Unbond"
      subHeader="Unbond and remove your node from the mixnet"
      okLabel="Unbond"
      onOk={handleGetFee}
      onClose={resetFeeState}
    >
      <ModalListItem label="Amount to unbond" value={`${node.bond.amount} ${node.bond.denom.toUpperCase()}`} />
      <ModalListItem label="Operator rewards" value={`${node.bond.amount} ${node.bond.denom.toUpperCase()}`} />
      <ModalFee isLoading={isFeeLoading} fee={fee} />
      <Typography>Tokens will be transferred to account you are logged in with now</Typography>
    </SimpleModal>
  );
};

export default Unbond;
