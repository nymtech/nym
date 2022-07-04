import * as React from 'react';
import { useContext, useEffect, useState } from 'react';
import { MajorCurrencyAmount, TransactionExecuteResult } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import { AppContext, BondedMixnode, urls } from '../../../../context';
import SummaryModal from './SummaryModal';
import { ConfirmationModal } from '../../../../components';

interface Props {
  mixnode: BondedMixnode;
  show: boolean;
  onClose: () => void;
}

const RedeemRewards = ({ mixnode, show, onClose }: Props) => {
  const [fee, setFee] = useState<MajorCurrencyAmount>({ amount: '0', denom: 'NYM' });
  const [step, setStep] = useState<1 | 2>(1);
  const [tx, setTx] = useState<TransactionExecuteResult>();

  const { network } = useContext(AppContext);

  useEffect(() => {
    setFee({ amount: '42', denom: 'NYM' }); // TODO fetch real fee amount
  }, []);

  const submit = () => {
    // TODO send request to redeem rewards
    setStep(2); // on success
    // setTx(requestResult)
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
        fee={fee as MajorCurrencyAmount}
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
    </>
  );
};

export default RedeemRewards;
