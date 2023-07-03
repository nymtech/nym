import React from 'react';
import { useNavigate } from 'react-router-dom';
import { useRegisterContext } from 'src/context';
import { SeedPhraseTemplate } from 'src/pages/templates';

export const AddAccount = () => {
  const { setUserMnemonic } = useRegisterContext();
  const navigate = useNavigate();

  const onNext = (seedPhrase: string) => {
    setUserMnemonic(seedPhrase);
    navigate('/user/accounts/name-account');
  };

  return <SeedPhraseTemplate onNext={onNext} />;
};
