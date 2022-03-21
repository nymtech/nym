import React, { createContext, useEffect, useMemo, useState } from 'react';
import { createMnemonic, signInWithMnemonic } from 'src/requests';
import { TMnemonicWords } from '../types';

export const SignInContext = createContext({} as TSignInContent);

export type TSignInContent = {
  error?: string;
  password: string;
  mnemonic: string;
  mnemonicWords: TMnemonicWords;
  setError: (err?: string) => void;
  setMnemonic: (mnc: string) => void;
  generateMnemonic: () => Promise<void>;
  validateMnemonic: () => Promise<void>;
  setPassword: (paswd: string) => void;
};

const mnemonicToArray = (mnemonic: string): TMnemonicWords =>
  mnemonic
    .split(' ')
    .reduce((a, c: string, index) => [...a, { name: c, index: index + 1, disabled: false }], [] as TMnemonicWords);

export const SignInProvider: React.FC = ({ children }) => {
  const [password, setPassword] = useState('');
  const [mnemonic, setMnemonic] = useState('');
  const [mnemonicWords, setMnemonicWords] = useState<TMnemonicWords>([]);
  const [error, setError] = useState<string>();

  const generateMnemonic = async () => {
    const mnemonicPhrase = await createMnemonic();
    setMnemonic(mnemonicPhrase);
  };

  const validateMnemonic = async () => {
    try {
      await signInWithMnemonic(mnemonic);
    } catch (e) {
      setError(e as string);
    }
  };

  useEffect(() => {
    if (mnemonic.length > 0) {
      const mnemonicArray = mnemonicToArray(mnemonic);
      setMnemonicWords(mnemonicArray);
    } else {
      setMnemonicWords([]);
    }
  }, [mnemonic]);

  return (
    <SignInContext.Provider
      value={useMemo(
        () => ({
          error,
          password,
          mnemonic,
          mnemonicWords,
          setError,
          setMnemonic,
          generateMnemonic,
          validateMnemonic,
          setPassword,
        }),
        [error, password, mnemonic, mnemonicWords],
      )}
    >
      {children}
    </SignInContext.Provider>
  );
};
