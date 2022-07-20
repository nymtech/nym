import * as React from 'react';
import { useContext, useEffect, useState } from 'react';
import { DecCoin, TransactionExecuteResult } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import { ErrorOutline } from '@mui/icons-material';
import ProfitMarginModal from './ProfitMarginModal';
import { AppContext, BondedMixnode, urls, useBondingContext } from '../../../../context';
import SummaryModal from './SummaryModal';
import { ConfirmationModal } from '../../../../components';

interface Props {
  mixnode: BondedMixnode;
  show: boolean;
  onClose: () => void;
}

// TODO fetch real estimated operator reward for 10% PM
const MOCK_ESTIMATED_OP_REWARD: DecCoin = { amount: '42', denom: 'nym' };

const NodeSettings = ({ mixnode, show, onClose }: Props) => {
  const [status, setStatus] = useState<'success' | 'error'>();
  const [profitMargin, setProfitMargin] = useState<number>();
  const [step, setStep] = useState<1 | 2>(1);
  const [tx, setTx] = useState<TransactionExecuteResult>();

  const { network } = useContext(AppContext);
  const { updateMixnode, error, fee, getFee } = useBondingContext();

  const submit = async () => {
    const txResult = await updateMixnode(profitMargin as number);
    if (txResult) {
      setStatus('success');
    } else {
      setStatus('error');
    }
    setTx(txResult);
  };

  const reset = () => {
    setProfitMargin(0);
    setStep(1);
    onClose();
  };

  return (
    <>
      <ProfitMarginModal
        open={show && step === 1}
        onClose={onClose}
        onConfirm={async (pm) => {
          setProfitMargin(pm);
          setStep(2);
        }}
        estimatedOpReward={MOCK_ESTIMATED_OP_REWARD}
        currentPm={mixnode.profitMargin}
      />
      <SummaryModal
        open={show && step === 2}
        onClose={reset}
        onConfirm={submit}
        onCancel={() => setStep(1)}
        currentPm={mixnode.profitMargin}
        newPm={profitMargin as number}
        fee={fee?.amount}
      />
      {status === 'success' && (
        <ConfirmationModal
          open={show}
          onClose={reset}
          onConfirm={reset}
          title="Operation successful"
          confirmButton="Done"
          maxWidth="xs"
        >
          <Typography sx={{ mb: 2 }}>This operation can take up to one hour to process</Typography>
          <Link href={`${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`} noIcon>
            View on blockchain
          </Link>
        </ConfirmationModal>
      )}
      {status === 'error' && (
        <ConfirmationModal
          open={show}
          onClose={reset}
          onConfirm={reset}
          title="Operation failed"
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

export default NodeSettings;
