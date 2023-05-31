import React, { useMemo, useState } from 'react';
import { ExtensionStorage } from '@nymproject/extension-storage';

const RegisterContext = React.createContext({} as TRegisterContext);

type TRegisterContext = {
  userPassword: string;
  userMnemonic: string;
  accountName: string;
  checkAccountName: () => Promise<boolean>;
  setUserPassword: (password: string) => void;
  setUserMnemonic: (mnemonic: string) => void;
  setAccountName: (name: string) => void;
  createAccount: (args: { mnemonic: string; password: string; accountName: string }) => Promise<void>;
  importAccount: () => Promise<string[]>;
};

export const RegisterContextProvider = ({ children }: { children: React.ReactNode }) => {
  const [userPassword, setUserPassword] = useState('');
  const [userMnemonic, setUserMnemonic] = useState('');
  const [accountName, setAccountName] = useState('');

  const createAccount = async ({
    mnemonic,
    password,
    accountName,
  }: {
    mnemonic: string;
    password: string;
    accountName: string;
  }) => {
    const storage = await new ExtensionStorage(password);
    await storage.store_mnemonic(accountName, mnemonic);
  };

  const importAccount = async () => {
    const storage = await new ExtensionStorage(userPassword);
    await storage.store_mnemonic(accountName, userMnemonic);
    const accounts = await storage.get_all_mnemonic_keys();
    return accounts;
  };

  const checkAccountName = async () => {
    return true;
  };

  const value = useMemo(
    () => ({
      userPassword,
      setUserPassword,
      userMnemonic,
      accountName,
      setAccountName,
      setUserMnemonic,
      createAccount,
      checkAccountName,
      importAccount,
    }),
    [userPassword, userMnemonic, accountName],
  );

  return <RegisterContext.Provider value={value}>{children}</RegisterContext.Provider>;
};

export const useRegisterContext = () => React.useContext(RegisterContext);
