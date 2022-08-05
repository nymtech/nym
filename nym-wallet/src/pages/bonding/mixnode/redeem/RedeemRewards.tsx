import * as React from 'react';
import { useContext, useEffect, useState } from 'react';
import { TransactionExecuteResult } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import { ErrorOutline } from '@mui/icons-material';
import { AppContext, TBondedMixnode, urls, useBondingContext } from '../../../../context';
import SummaryModal from './SummaryModal';
import { ConfirmationModal } from '../../../../components';

interface Props {
  mixnode: TBondedMixnode;
  show: boolean;
  onClose: () => void;
}

const RedeemRewards = ({ mixnode, show, onClose }: Props) => {
  const [step, setStep] = useState<1 | 2>(1);
  const [tx, setTx] = useState<TransactionExecuteResult>();

  const { network } = useContext(AppContext);
  const { redeemRewards, error } = useBondingContext();

  const submit = async () => {
    const txResult = await redeemRewards();
    if (txResult) {
      setStep(2);
    }
  };

  const reset = () => {
    setStep(1);
    onClose();
  };

  return (
    <>
      <SummaryModal
        open={show && step === 1}
        onClose={reset}
        onConfirm={submit}
        onCancel={reset}
        rewards={mixnode.operatorRewards}
      />
      <ConfirmationModal
        open={show && step === 2}
        onClose={reset}
        onConfirm={reset}
        title="Rewards redemption succesfull"
        confirmButton="Done"
        maxWidth="xs"
      >
        <Typography sx={{ mb: 2 }}>This operation can take up to one hour to process</Typography>
        <Link href={`${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`} noIcon>
          View on blockchain
        </Link>
      </ConfirmationModal>
      {error && (
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

export default RedeemRewards;
