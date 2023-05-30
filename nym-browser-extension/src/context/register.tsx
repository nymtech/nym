import React, { useMemo, useState } from 'react';
import { ExtensionStorage } from '@nymproject/extension-storage';

const RegisterContext = React.createContext({} as TRegisterContext);

type TRegisterContext = {
  userPassword: string;
  userMnemonic: string;
  setUserPassword: (password: string) => void;
  setUserMnemonic: (mnemonic: string) => void;
  createNewAccount: (mnemonic: string) => Promise<void>;
  importExistingAccount: (password: string) => Promise<void>;
};

export const RegisterContextProvider = ({ children }: { children: React.ReactNode }) => {
  const [userPassword, setUserPassword] = useState('');
  const [userMnemonic, setUserMnemonic] = useState('');

  const createAccount = async (mnemonic: string, password: string) => {
    const storage = await new ExtensionStorage(password);
    await storage.store_mnemonic('Default account', mnemonic);
    localStorage.setItem('nym-browser-extension', 'true');
  };

  const createNewAccount = async (mnemonic: string) => createAccount(mnemonic, userPassword);

  const importExistingAccount = async (password: string) => createAccount(userMnemonic, password);

  const value = useMemo(
    () => ({
      userPassword,
      setUserPassword,
      userMnemonic,
      setUserMnemonic,
      createNewAccount,
      importExistingAccount,
    }),
    [userPassword, userMnemonic],
  );

  return <RegisterContext.Provider value={value}>{children}</RegisterContext.Provider>;
};

export const useRegisterContext = () => React.useContext(RegisterContext);
