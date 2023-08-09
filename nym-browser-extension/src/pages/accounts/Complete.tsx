import React from 'react';
import { useNavigate } from 'react-router-dom';
import { SetupCompleteTemplate } from 'src/pages/templates/Complete';

export const SetupComplete = () => {
  const navigate = useNavigate();
  const handleOnDone = () => {
    navigate('/user/accounts');
  };

  return (
    <SetupCompleteTemplate title="You're all set!" description="Account successfully imported" onDone={handleOnDone} />
  );
};
