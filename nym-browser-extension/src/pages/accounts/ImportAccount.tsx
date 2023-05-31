import React from 'react';
import { ImportAccountTemplate } from '../templates';
import { useLocation, useNavigate } from 'react-router-dom';
import { useRegisterContext } from 'src/context/register';

export const ImportAccount = () => {
  const { userMnemonic, setUserMnemonic } = useRegisterContext();
  const location = useLocation();
  const navigate = useNavigate();

  const handleOnNext = () => {
    navigate(`${location.pathname}/name-account`);
  };

  return (
    <ImportAccountTemplate userMnemonic={userMnemonic} onChangeUserMnemonic={setUserMnemonic} onNext={handleOnNext} />
  );
};
