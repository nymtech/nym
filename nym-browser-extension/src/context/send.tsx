import React, { useMemo, useState } from 'react';
import { DecCoin } from '@nymproject/types';
import { useNavigate } from 'react-router-dom';
import { nymToUnym } from 'src/utils/coin';
import { TTransaction } from 'src/types';
import { Fee, useGetFee } from 'src/hooks/useGetFee';
import { createFeeObject } from 'src/utils/fee';
import { useAppContext } from './app';

type TSendContext = {
  address?: string;
  amount?: DecCoin;
  transaction?: TTransaction;
  fee?: Fee;
  handleChangeAddress: (address?: string) => void;
  handleChangeAmount: (amount?: DecCoin) => void;
  handleSend: () => void;
  resetTx: () => void;
  onDone: () => void;
  handleGetFee: (address: string, amount: string) => Promise<void>;
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

  const handleGetFee = async (addressVal: string, amountVal: string) => {
    const unym = nymToUnym(Number(amountVal));

    if (client) {
      // client loses its 'this' context when passing the method
      // TODO find a better way of doing this.
      getFee(client.simulateSend.bind(client), {
        signingAddress: client.address,
        from: client.address,
        to: addressVal,
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
        const response = await client.send(
          address,
          [{ amount: unyms.toString(), denom: minorDenom }],
          createFeeObject(fee?.unym),
        );

        setTransaction({ status: 'success', type: 'send', txHash: response?.transactionHash });
      } catch (e) {
        setTransaction({
          status: 'error',
          type: 'send',
          message: e instanceof Error ? e.message : 'Error making send transaction. Please try again',
        });
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
