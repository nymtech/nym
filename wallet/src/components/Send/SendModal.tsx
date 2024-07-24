import { useContext, useEffect, useState } from 'react';
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

export const SendModal = ({ onClose, hasStorybookStyles }: { onClose: () => void; hasStorybookStyles?: object }) => {
  const [toAddress, setToAddress] = useState<string>('');
  const [amount, setAmount] = useState<DecCoin>();
  const [modal, setModal] = useState<'send' | 'send details'>('send');
  const [error, setError] = useState<string>();
  const [sendError, setSendError] = useState(false);
  const [gasError, setGasError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);
  const [userFees, setUserFees] = useState<DecCoin>();
  const [memo, setMemo] = useState<string>('');
  const [txDetails, setTxDetails] = useState<TTransactionDetails>();
  const [showMoreOptions, setShowMoreOptions] = useState(false);

  const { clientDetails, userBalance, network } = useContext(AppContext);
  const { fee, getFee, feeError, setFeeManually } = useGetFee();

  useEffect(() => {
    if (userFees?.amount.length === 0) {
      setUserFees(undefined);
    }
  }, [userFees]);

  useEffect(() => {
    if (!showMoreOptions) {
      setUserFees(undefined);
      setMemo('');
    }
  }, [showMoreOptions]);

  // removes any zero-width spaces and trailing white space
  const sanitizeAddress = (address: string) => address.replace(/[\u200B-\u200D\uFEFF]/g, '').trim();

  const handleOnNext = async () => {
    if (amount) {
      setIsLoading(true);
      setError(undefined);
      try {
        if (userFees) {
          await setFeeManually(userFees);
        } else {
          await getFee(simulateSend, { address: toAddress, amount, memo });
        }
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
      const txResponse = await send({ amount: val, address: to, memo: memo || '', fee: fee?.fee });
      setTxDetails({
        amount: `${amount?.amount} ${clientDetails?.display_mix_denom.toUpperCase()}`,
        txUrl: `${urls(network).blockExplorer}/transaction/${txResponse.tx_hash}`,
      });
    } catch (e) {
      Console.error(e as string);
      if (/Raw log: out of gas/.test(e as string)) {
        setGasError('Specified fee was too small. Please increase the amount and try again');
      } else {
        setSendError(true);
      }
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading) return <LoadingModal />;

  if (sendError) return <SendErrorModal onClose={onClose} error={feeError} />;

  if (gasError) {
    return <SendErrorModal onClose={onClose} error={gasError} />;
  }

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
        memo={memo}
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
      userFees={userFees}
      memo={memo}
      showMore={showMoreOptions}
      onAmountChange={(value) => setAmount(value)}
      onAddressChange={(value) => setToAddress(sanitizeAddress(value))}
      onUserFeesChange={(value) => setUserFees(value)}
      onMemoChange={(value) => setMemo(value)}
      setShowMore={setShowMoreOptions}
      {...hasStorybookStyles}
    />
  );
};
