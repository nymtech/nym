import React, { useMemo, useState } from 'react';
import { DecCoin } from '@nymproject/types';
import { useNavigate } from 'react-router-dom';
import { nymToUnym } from 'src/utils/coin';
import { TTransaction } from 'src/types';
import { useAppContext } from './app';

type TSendContext = {
  address?: string;
  amount?: DecCoin;
  transaction?: TTransaction;
  handleChangeAddress: (address?: string) => void;
  handleChangeAmount: (amount?: DecCoin) => void;
  handleSend: () => void;
  resetTx: () => void;
  onDone: () => void;
};

const SendContext = React.createContext({} as TSendContext);

export const SendProvider = ({ children }: { children: React.ReactNode }) => {
  const [address, setAddress] = useState<string>();
  const [amount, setAmount] = useState<DecCoin>();
  const [transaction, setTransaction] = useState<TTransaction>();

  const { client, minorDenom } = useAppContext();
  const navigate = useNavigate();

  const handleChangeAddress = (_address?: string) => setAddress(_address);

  const handleChangeAmount = (_amount?: DecCoin) => setAmount(_amount);

  const handleSend = async () => {
    setTransaction({ status: 'loading', type: 'send' });
    let unyms;

    if (!Number(amount?.amount)) {
      setTransaction({ status: 'error', type: 'send', message: 'Amount is not a valid number' });
    }

    if (amount) {
      unyms = nymToUnym(amount.amount);
    }

    if (address && unyms) {
      try {
        const response = await client?.send(address, [{ amount: unyms, denom: minorDenom }]);
        setTransaction({ status: 'success', type: 'send', txHash: response?.transactionHash });
      } catch (e) {
        setTransaction({ status: 'error', type: 'send', message: e as string });
      }
    }
  };

  const resetTx = () => {
    setTransaction(undefined);
  };

  const onDone = () => {
    navigate('/user/balance');
  };

  const value = useMemo<TSendContext>(
    () => ({
      address,
      amount,
      transaction,
      handleChangeAddress,
      handleChangeAmount,
      handleSend,
      resetTx,
      onDone,
    }),
    [address, amount, transaction],
  );

  return <SendContext.Provider value={value}>{children}</SendContext.Provider>;
};

export const useSendContext = () => React.useContext(SendContext);
