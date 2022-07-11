import * as React from 'react';
import { useContext, useEffect, useState } from 'react';
import { MajorCurrencyAmount } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import { ErrorOutline } from '@mui/icons-material';
import { AppContext, BondedGateway, BondedMixnode, urls, useBondingContext } from '../../../context';
import SummaryModal from './SummaryModal';
import { ConfirmationModal } from '../../../components';
import { LoadingModal } from '../../../components/Modals/LoadingModal';
import { NodeType } from '../types';

interface Props {
  node: BondedMixnode | BondedGateway;
  show: boolean;
  onClose: () => void;
}

type UnbondStatus = 'success' | 'error';

const Unbond = ({ node, show, onClose }: Props) => {
  const [step, setStep] = useState<1 | 2>(1);
  const [txHash, setTxHash] = useState<string>();
  const [status, setStatus] = useState<UnbondStatus>();
  const [nodeType, setNodeType] = useState<NodeType>('mixnode');

  const { network } = useContext(AppContext);
  const { fee, getFee, resetFeeState, feeLoading, feeError, loading, unbondMixnode, unbondGateway, error } =
    useBondingContext();

  useEffect(() => {
    if (error || feeError) {
      setStatus('error');
    }
  }, [error, feeError]);

  useEffect(() => {
    if ('profitMarfin' in node) {
      setNodeType('mixnode');
    } else {
      setNodeType('gateway');
    }
  }, [node]);

  const unbond = async () => {
    let tx;
    if (nodeType === 'mixnode') {
      tx = await unbondMixnode();
    } else {
      tx = await unbondGateway();
    }
    if (!tx) {
      setStatus('error');
    }
    setStatus('success');
    setTxHash(tx?.transaction_hash);
    return tx;
  };

  const fetchFee = async () => {
    if (nodeType === 'mixnode') {
      await getFee('unbondMixnode', {});
    } else {
      await getFee('unbondGateway', {});
    }
  };

  useEffect(() => {
    fetchFee();
  }, [node]);

  const submit = async () => {
    if (status === 'error') {
      // Fetch fee failed
      return;
    }
    unbond();
    resetFeeState();
    setStep(2);
  };

  const reset = () => {
    setStep(1);
    onClose();
  };

  if (feeLoading || loading) return <LoadingModal />;

  return (
    <>
      <SummaryModal
        open={show && step === 1}
        onClose={reset}
        onConfirm={submit}
        onCancel={reset}
        bond={node.bond}
        rewards={nodeType === 'mixnode' ? (node as BondedMixnode).operatorRewards : undefined}
        fee={fee?.amount}
      />
      {status === 'success' && (
        <ConfirmationModal
          open={show && step === 2}
          onClose={reset}
          onConfirm={reset}
          title="Unbonding succesfull"
          confirmButton="Done"
          maxWidth="xs"
        >
          <Typography sx={{ mb: 2 }}>This operation can take up to one hour to process</Typography>
          <Link href={`${urls(network).blockExplorer}/transaction/${txHash}`} noIcon>
            View on blockchain
          </Link>
        </ConfirmationModal>
      )}
      {status === 'error' && (
        <ConfirmationModal
          open={show}
          onClose={reset}
          onConfirm={reset}
          title="Unbonding failed"
          confirmButton="Done"
          maxWidth="xs"
        >
          <Typography variant="caption">Error: {error}</Typography>
          <ErrorOutline color="error" />
        </ConfirmationModal>
      )}
    </>
  );
};

export default Unbond;
