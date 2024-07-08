import React, { createContext, useEffect, useMemo, useState } from 'react';
import { createMnemonic } from '@src/requests';
import { TMnemonicWords } from '@src/pages/auth/types';

export const AuthContext = createContext({} as TAuthContext);

export type TAuthContext = {
  error?: string;
  password: string;
  mnemonic: string;
  mnemonicWords: TMnemonicWords;
  setError: (err?: string) => void;
  setMnemonic: (mnc: string) => void;
  generateMnemonic: () => Promise<void>;
  setPassword: (pwd: string) => void;
  resetState: () => void;
};

const mnemonicToArray = (mnemonic: string): TMnemonicWords =>
  mnemonic
    .split(' ')
    .reduce((a, c: string, index) => [...a, { name: c, index: index + 1, disabled: false }], [] as TMnemonicWords);

export const AuthProvider: FCWithChildren = ({ children }) => {
  const [password, setPassword] = useState('');
  const [mnemonic, setMnemonic] = useState('');
  const [mnemonicWords, setMnemonicWords] = useState<TMnemonicWords>([]);
  const [error, setError] = useState<string>();

  const generateMnemonic = async () => {
    const mnemonicPhrase = await createMnemonic();
    setMnemonic(mnemonicPhrase);
  };

  useEffect(() => {
    if (mnemonic.length > 0) {
      const mnemonicArray = mnemonicToArray(mnemonic);
      setMnemonicWords(mnemonicArray);
    } else {
      setMnemonicWords([]);
    }
  }, [mnemonic]);

  const resetState = () => {
    setPassword('');
    setMnemonic('');
  };

  return (
    <AuthContext.Provider
      value={useMemo(
        () => ({
          error,
          password,
          mnemonic,
          mnemonicWords,
          setError,
          setMnemonic,
          generateMnemonic,
          setPassword,
          resetState,
        }),
        [error, password, mnemonic, mnemonicWords],
      )}
    >
      {children}
    </AuthContext.Provider>
  );
};
