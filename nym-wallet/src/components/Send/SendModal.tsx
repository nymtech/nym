import React, { useContext, useEffect, useState } from 'react';
import Big from 'big.js';
import { DecCoin } from '@nymproject/types';
import { AppContext, urls } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { send } from 'src/requests';
import { Console } from 'src/utils/console';
import { simulateSend } from 'src/requests/simulate';
import { validateNymAddress } from 'src/utils/validateNymAddress';
import { LoadingModal } from '../Modals/LoadingModal';
import { SendDetailsModal } from './SendDetailsModal';
import { SendErrorModal } from './SendErrorModal';
import { SendInputModal } from './SendInputModal';
import { SendSuccessModal } from './SendSuccessModal';
import { TTransactionDetails } from './types';

/** Extra NYM left in the account after Max to absorb minor gas / rounding drift. */
const MAX_SEND_FEE_RESERVE = '0.01';
const MIN_SEND_AMOUNT = '0.000001';

export const SendModal = ({ onClose }: { onClose: () => void }) => {
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
  const [amountFieldKey, setAmountFieldKey] = useState(0);
  const [maxAmountLoading, setMaxAmountLoading] = useState(false);

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

  const handleMaxAmount = async () => {
    setError(undefined);
    if (!userBalance.balance?.amount) {
      setError('Balance unavailable.');
      return;
    }
    if (!validateNymAddress(toAddress)) {
      setError('Enter a valid recipient address to estimate network fees for Max.');
      return;
    }

    const balDec = userBalance.balance.amount;
    setMaxAmountLoading(true);
    try {
      let feeDisplay: string;

      if (showMoreOptions && userFees?.amount && String(userFees.amount).trim() !== '') {
        if (!Number(userFees.amount)) {
          setError('Set a valid custom fee or turn off More options.');
          return;
        }
        feeDisplay = userFees.amount;
      } else {
        const probe = await simulateSend({
          address: toAddress,
          amount: balDec,
          memo: memo || '',
        });
        if (!probe.amount?.amount) {
          setError('Could not estimate network fee. Try again or set a custom fee under More options.');
          return;
        }
        feeDisplay = probe.amount.amount;
      }

      const balanceBig = new Big(balDec.amount);
      const feeBig = new Big(feeDisplay);
      const reserveBig = new Big(MAX_SEND_FEE_RESERVE);
      let maxBig = balanceBig.minus(feeBig).minus(reserveBig);

      if (maxBig.lte(0)) {
        setError(
          'Balance is too low to send after fees and reserve. Add funds, lower the custom fee, or try again later.',
        );
        return;
      }

      if (!(showMoreOptions && userFees?.amount && String(userFees.amount).trim() !== '')) {
        const refined = await simulateSend({
          address: toAddress,
          amount: { amount: maxBig.toString(), denom: balDec.denom },
          memo: memo || '',
        });
        if (refined.amount?.amount) {
          maxBig = balanceBig.minus(new Big(refined.amount.amount)).minus(reserveBig);
        }
      }

      if (maxBig.lte(0) || maxBig.lt(new Big(MIN_SEND_AMOUNT))) {
        setError(
          `Max sendable amount is below the minimum (${MIN_SEND_AMOUNT} ${(
            clientDetails?.display_mix_denom || 'nym'
          ).toUpperCase()}).`,
        );
        return;
      }

      const rounded = maxBig.round(6, 0);
      let amountStr = rounded.toFixed(6).replace(/\.?0+$/, '');
      if (amountStr === '' || amountStr === '.') {
        amountStr = MIN_SEND_AMOUNT;
      }

      setAmount({ amount: amountStr, denom: balDec.denom });
      setAmountFieldKey((k) => k + 1);
    } catch (e) {
      setError(String(e));
    } finally {
      setMaxAmountLoading(false);
    }
  };

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
        txUrl: `${urls(network).blockExplorer}/tx/${txResponse.tx_hash}`,
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
      amountFieldKey={amountFieldKey}
      onMaxAmount={handleMaxAmount}
      maxAmountLoading={maxAmountLoading}
    />
  );
};
