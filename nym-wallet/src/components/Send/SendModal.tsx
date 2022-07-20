import React, { useContext, useState } from 'react';
import { DecCoin } from '@nymproject/types';
import { AppContext, urls } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { send } from 'src/requests';
import { Console } from 'src/utils/console';
import { simulateSend } from 'src/requests/simulate';
import { LoadingModal } from '../Modals/LoadingModal';
import { SendDetailsModal } from './SendDetailsModal';
import { SendErrorModal } from './SendErrorModal';
import { SendInputModal } from './SendInputModal';
import { SendSuccessModal } from './SendSuccessModal';
import { TTransactionDetails } from './types';

export const SendModal = ({ onClose, hasStorybookStyles }: { onClose: () => void; hasStorybookStyles?: {} }) => {
  const [toAddress, setToAddress] = useState<string>('');
  const [amount, setAmount] = useState<DecCoin>();
  const [modal, setModal] = useState<'send' | 'send details'>('send');
  const [error, setError] = useState<string>();
  const [sendError, setSendError] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [txDetails, setTxDetails] = useState<TTransactionDetails>();

  const { clientDetails, userBalance, network } = useContext(AppContext);
  const { fee, getFee } = useGetFee();

  const handleOnNext = async () => {
    if (amount) {
      setIsLoading(true);
      setError(undefined);
      try {
        await getFee(simulateSend, { address: toAddress, amount });
        setModal('send details');
      } catch (e) {
        setError(e as string);
      } finally {
        setIsLoading(false);
      }
    } else {
      setError('An amount is required');
    }
  };

  const handleSend = async ({ val, to }: { val: DecCoin; to: string }) => {
    setIsLoading(true);
    setError(undefined);
    try {
      const txResponse = await send({ amount: val, address: to, memo: '', fee: fee?.fee });
      setTxDetails({
        amount: `${amount?.amount} ${clientDetails?.display_mix_denom.toUpperCase()}`,
        txUrl: `${urls(network).blockExplorer}/transaction/${txResponse.tx_hash}`,
      });
    } catch (e) {
      Console.error(e as string);
      setSendError(true);
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading) return <LoadingModal />;

  if (sendError) return <SendErrorModal onClose={onClose} />;

  if (txDetails) return <SendSuccessModal txDetails={txDetails} onClose={onClose} />;

  if (modal === 'send details')
    return (
      <SendDetailsModal
        fromAddress={clientDetails?.client_address}
        toAddress={toAddress}
        amount={amount}
        fee={fee}
        onClose={onClose}
        onPrev={() => setModal('send')}
        onSend={handleSend}
        denom={clientDetails?.display_mix_denom || 'nym'}
        {...hasStorybookStyles}
      />
    );

  return (
    <SendInputModal
      fromAddress={clientDetails?.client_address}
      toAddress={toAddress}
      amount={amount}
      balance={userBalance.balance?.printable_balance}
      onClose={onClose}
      onNext={handleOnNext}
      error={error}
      denom={clientDetails?.display_mix_denom}
      onAmountChange={(value) => setAmount(value)}
      onAddressChange={(value) => setToAddress(value)}
      denom={denom}
      {...hasStorybookStyles}
    />
  );
};
