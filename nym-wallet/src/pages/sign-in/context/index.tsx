import React, { createContext, useEffect, useMemo, useState } from 'react';
import { useHistory } from 'react-router-dom';
import { createMnemonic } from 'src/requests';
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

  const history = useHistory();

  const generateMnemonic = async () => {
    const mnemonicPhrase = await createMnemonic();
    setMnemonic(mnemonicPhrase);
  };

  useEffect(() => {
    history.push('/welcome');
  }, []);

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
          setPassword,
        }),
        [error, password, mnemonic, mnemonicWords],
      )}
    >
      {children}
    </SignInContext.Provider>
  );
};
