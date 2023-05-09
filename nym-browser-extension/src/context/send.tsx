import React, { useMemo, useState } from 'react';
import { DecCoin } from '@nymproject/types';
import { useNavigate } from 'react-router-dom';
import { nymToUnym } from 'src/utils/coin';
import { TTransaction } from 'src/types';
import { useGetFee } from 'src/hooks/useGetFee';
import { useAppContext } from './app';

type TSendContext = {
  address?: string;
  amount?: DecCoin;
  transaction?: TTransaction;
  fee?: number;
  handleChangeAddress: (address?: string) => void;
  handleChangeAmount: (amount?: DecCoin) => void;
  handleSend: () => void;
  resetTx: () => void;
  onDone: () => void;
  handleGetFee: () => Promise<void>;
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

  const { getFee, fee } = useGetFee();

  const handleGetFee = async () => {
    let unym: number | undefined;

    if (amount) {
      unym = nymToUnym(Number(amount?.amount));
    }

    if (address && unym && client) {
      getFee(client.simulateSend, {
        signingAddress: client.address,
        from: client.address,
        to: address,
        amount: [{ amount: unym.toString(), denom: minorDenom }],
      });
    }
  };

  const handleSend = async () => {
    setTransaction({ status: 'loading', type: 'send' });
    let unyms;

    if (!Number(amount?.amount)) {
      setTransaction({ status: 'error', type: 'send', message: 'Amount is not a valid number' });
    }

    if (amount) {
      unyms = nymToUnym(Number(amount.amount));
    }

    if (client && address && unyms) {
      try {
        const response = await client.send(address, [{ amount: unyms.toString(), denom: minorDenom }], fee);
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
      fee,
      handleChangeAddress,
      handleChangeAmount,
      handleSend,
      resetTx,
      onDone,
      handleGetFee,
    }),
    [address, amount, transaction, fee],
  );

  return <SendContext.Provider value={value}>{children}</SendContext.Provider>;
};

export const useSendContext = () => React.useContext(SendContext);
