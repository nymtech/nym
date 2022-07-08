import * as React from 'react';
import { useContext, useEffect, useState } from 'react';
import { MajorCurrencyAmount } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { Typography } from '@mui/material';
import { ErrorOutline } from '@mui/icons-material';
import { AppContext, BondedGateway, BondedMixnode, urls } from '../../../../context';
import SummaryModal from './SummaryModal';
import { ConfirmationModal } from '../../../../components';
import {
  simulateUnbondGateway,
  simulateVestingUnbondGateway,
  unbondGateway,
  vestingUnbondGateway,
} from '../../../../requests';
import { useCheckOwnership } from '../../../../hooks/useCheckOwnership';
import { useGetFee } from '../../../../hooks/useGetFee';
import { LoadingModal } from '../../../../components/Modals/LoadingModal';

interface Props {
  node: BondedGateway;
  show: boolean;
  onClose: () => void;
}

type UnbondStatus = 'success' | 'error';

const Unbond = ({ node, show, onClose }: Props) => {
  const [isLoading, setIsLoading] = useState(false);
  const [step, setStep] = useState<1 | 2>(1);
  const [txHash, setTxHash] = useState<string>();
  const [status, setStatus] = useState<UnbondStatus>();
  const [error, setError] = useState<string>();

  const { fee, getFee, resetFeeState, isFeeLoading } = useGetFee();

  const { network } = useContext(AppContext);
  const { checkOwnership, ownership } = useCheckOwnership();

  const isVesting = Boolean(ownership.vestingPledge);

  const unbond = async () => {
    let tx;
    setIsLoading(true);
    try {
      if (isVesting) tx = await vestingUnbondGateway(fee?.fee);
      if (!isVesting) tx = await unbondGateway(fee?.fee);
      setTxHash(tx?.transaction_hash);
      setStatus('success');
    } catch (err: any) {
      setStatus('error');
      setError(err as string);
    } finally {
      await checkOwnership();
      setIsLoading(false);
    }
  };

  const fetchFee = async () => {
    try {
      if (isVesting) await getFee(simulateVestingUnbondGateway, {});
      if (!isVesting) await getFee(simulateUnbondGateway, {});
    } catch (e: any) {
      setStatus('error');
      setError(e as string);
    }
  };

  useEffect(() => {
    fetchFee();
  }, [node, isVesting]);

  const submit = () => {
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

  if (isFeeLoading || isLoading) return <LoadingModal />;

  return (
    <>
      <SummaryModal
        open={show && step === 1}
        onClose={reset}
        onConfirm={submit}
        onCancel={reset}
        bond={node.bond}
        rewards={(node as BondedMixnode).operatorRewards}
        fee={fee?.amount as MajorCurrencyAmount}
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
