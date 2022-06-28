import * as React from 'react';
import { useContext, useEffect, useState } from 'react';
import { MajorCurrencyAmount, TransactionExecuteResult } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import { AppContext, BondedMixnode, urls } from '../../../../context';
import SummaryModal from './SummaryModal';
import { SimpleDialog } from '../../components';
import BondModal from './BondModal';

interface Props {
  mixnode: BondedMixnode;
  show: boolean;
  onClose: () => void;
}

const BondMore = ({ mixnode, show, onClose }: Props) => {
  const [addBond, setAddBond] = useState<MajorCurrencyAmount>({ amount: '0', denom: 'NYM' });
  const [signature, setSignature] = useState<string>();
  const [fee, setFee] = useState<MajorCurrencyAmount>({ amount: '0', denom: 'NYM' });
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [tx, setTx] = useState<TransactionExecuteResult>();

  const { network } = useContext(AppContext);

  useEffect(() => {
    setFee({ amount: '42', denom: 'NYM' }); // TODO fetch real fee amount
  }, [addBond]);

  const submit = () => {
    // TODO send request to update bond
    setStep(3); // on success
    // setTx(requestResult)
  };

  const reset = () => {
    setAddBond({ amount: '0', denom: 'NYM' });
    setSignature('');
    setStep(1);
    onClose();
  };

  return (
    <>
      <BondModal
        open={show && step === 1}
        onClose={onClose}
        onConfirm={async (bond, sig) => {
          setAddBond(bond);
          setSignature(sig);
          setStep(2);
        }}
        currentBond={mixnode.bond}
      />
      <SummaryModal
        open={show && step === 2}
        onClose={reset}
        onConfirm={submit}
        onCancel={() => setStep(1)}
        currentBond={mixnode.bond}
        addBond={addBond}
        fee={fee as MajorCurrencyAmount}
      />
      <SimpleDialog
        open={show && step === 3}
        onClose={reset}
        onConfirm={reset}
        title="Bonding successful"
        confirmButton="Done"
        maxWidth="xs"
        sx={{ textAlign: 'center' }}
      >
        <Typography sx={{ mb: 2 }}>This operation can take up to one hour to process</Typography>
        <Link href={`${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`} noIcon>
          View on blockchain
        </Link>
      </SimpleDialog>
    </>
  );
};

export default BondMore;
