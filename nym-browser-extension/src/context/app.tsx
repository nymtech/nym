import React, { useEffect, useMemo, useState } from 'react';
import ValidatorClient from '@nymproject/nym-validator-client';
import { connectToValidator } from 'src/validator-client';
import { unymToNym } from 'src/utils/coin';

type TAppContext = {
  client?: ValidatorClient;
  balance?: string;
  denom: 'NYM';
  minorDenom: 'unym';
  handleUnlockWallet: (password: string) => void;
  getBalance: () => void;
};

type TBalanceInNYMs = string;

const AppContext = React.createContext({} as TAppContext);

export const AppProvider = ({ children }: { children: React.ReactNode }) => {
  const [client, setClient] = useState<ValidatorClient>();
  const [balance, setBalance] = useState<TBalanceInNYMs>();
  const denom = 'NYM';
  const minorDenom = 'unym';

  const handleUnlockWallet = async (password: string) => {
    const c = await connectToValidator(password);
    setClient(c);
  };

  const getBalance = async () => {
    const bal = await client?.getBalance(client.address);

    if (bal) {
      const nym = unymToNym(bal?.amount);
      setBalance(nym);
    }
  };

  useEffect(() => {
    getBalance();
  }, [client]);

  const value = useMemo<TAppContext>(
    () => ({ client, balance, denom, minorDenom, handleUnlockWallet, getBalance }),
    [client, balance, denom, minorDenom],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
};

export const useAppContext = () => React.useContext(AppContext);
