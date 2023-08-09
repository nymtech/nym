import React from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useRegisterContext } from 'src/context/register';
import { ImportAccountTemplate } from '../templates/ImportAccount';

export const ImportAccount = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const { setUserMnemonic, userMnemonic } = useRegisterContext();

  const handleNext = async () => {
    navigate(`${location.pathname}/create-password`);
  };

  return (
    <ImportAccountTemplate userMnemonic={userMnemonic} onChangeUserMnemonic={setUserMnemonic} onNext={handleNext} />
  );
};
