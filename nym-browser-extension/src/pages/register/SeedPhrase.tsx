import React from 'react';
import { useNavigate } from 'react-router-dom';
import { useRegisterContext } from 'src/context/register';
import { SeedPhraseTemplate } from '../templates/SeedPhrase';

export const SeedPhrase = () => {
  const navigate = useNavigate();

  const { createAccount, userPassword } = useRegisterContext();

  const handleEncryptSeedPhrase = async (seedPhrase: string) => {
    await createAccount({ mnemonic: seedPhrase, password: userPassword, accName: 'Default account' });
    navigate('/register/complete');
  };

  return <SeedPhraseTemplate onNext={handleEncryptSeedPhrase} />;
};
