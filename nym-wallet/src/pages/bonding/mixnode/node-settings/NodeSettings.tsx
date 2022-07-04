import * as React from 'react';
import { useContext, useEffect, useState } from 'react';
import { MajorCurrencyAmount, TransactionExecuteResult } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import ProfitMarginModal from './ProfitMarginModal';
import { AppContext, BondedMixnode, urls } from '../../../../context';
import SummaryModal from './SummaryModal';
import { ConfirmationModal } from '../../../../components';

interface Props {
  mixnode: BondedMixnode;
  show: boolean;
  onClose: () => void;
}

// TODO fetch real estimated operator reward for 10% PM
const MOCK_ESTIMATED_OP_REWARD: MajorCurrencyAmount = { amount: '42', denom: 'NYM' };

const NodeSettings = ({ mixnode, show, onClose }: Props) => {
  const [profitMargin, setProfitMargin] = useState<number>();
  const [fee, setFee] = useState<MajorCurrencyAmount>({ amount: '0', denom: 'NYM' });
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [tx, setTx] = useState<TransactionExecuteResult>();

  const { network } = useContext(AppContext);

  useEffect(() => {
    setFee({ amount: '42', denom: 'NYM' }); // TODO fetch real fee amount
  }, [profitMargin]);

  const submit = () => {
    // TODO send request to update profit margin
    setStep(3); // on success
    // setTx(requestResult)
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
        fee={fee as MajorCurrencyAmount}
      />
      <ConfirmationModal
        open={show && step === 3}
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
    </>
  );
};

export default NodeSettings;
