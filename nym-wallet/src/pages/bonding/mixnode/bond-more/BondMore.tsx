import * as React from 'react';
import { useContext, useState } from 'react';
import { DecCoin, TransactionExecuteResult } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import { ErrorOutline } from '@mui/icons-material';
import { AppContext, BondedMixnode, urls, useBondingContext } from '../../../../context';
import SummaryModal from './SummaryModal';
import { ConfirmationModal } from '../../../../components';
import BondModal from './BondModal';

interface Props {
  mixnode: BondedMixnode;
  show: boolean;
  onClose: () => void;
}

const BondMore = ({ mixnode, show, onClose }: Props) => {
  const [addBond, setAddBond] = useState<DecCoin>({ amount: '0', denom: 'nym' });
  const [signature, setSignature] = useState<string>();
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [tx, setTx] = useState<TransactionExecuteResult>();

  const { network } = useContext(AppContext);
  const { bondMore, error } = useBondingContext();

  const submit = async () => {
    const txResult = await bondMore(signature as string, addBond);
    if (txResult) {
      setStep(3);
    }
    setTx(txResult);
  };

  const reset = () => {
    setAddBond({ amount: '0', denom: 'nym' });
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
      />
      <ConfirmationModal
        open={show && step === 3 && !error}
        onClose={reset}
        onConfirm={reset}
        title="Bonding successful"
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

export default BondMore;
