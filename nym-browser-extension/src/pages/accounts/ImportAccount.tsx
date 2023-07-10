import React from 'react';
import { useNavigate } from 'react-router-dom';
import { useRegisterContext } from 'src/context/register';
import { ImportAccountTemplate } from '../templates';

export const ImportAccount = () => {
  const { userMnemonic, setUserMnemonic } = useRegisterContext();
  const navigate = useNavigate();

  const handleOnNext = () => {
    navigate('/user/accounts/name-account');
  };

  return (
    <ImportAccountTemplate userMnemonic={userMnemonic} onChangeUserMnemonic={setUserMnemonic} onNext={handleOnNext} />
  );
};
