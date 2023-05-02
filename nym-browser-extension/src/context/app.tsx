import React, { useEffect, useState } from 'react';
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
    const balance = await client?.getBalance(client.address);

    if (balance) {
      const nym = unymToNym(balance?.amount);
      setBalance(nym);
    }
  };

  useEffect(() => {
    getBalance();
  }, [client]);

  return (
    <AppContext.Provider value={{ client, balance, denom, minorDenom, handleUnlockWallet, getBalance }}>
      {children}
    </AppContext.Provider>
  );
};

export const useAppContext = () => React.useContext(AppContext);
