import React from 'react';
import { useCreatePassword } from 'src/hooks/useCreatePassword';
import { CreatePassword } from 'src/components/register/CreatePassword';
import { useRegisterContext } from 'src/context/register';

export const CreatePasswordOnNewAccount = ({ onNext }: { onNext: () => void }) => {
  const passwordState = useCreatePassword();
  const { setUserPassword } = useRegisterContext();

  const handleCreateAccount = async () => {
    await setUserPassword(passwordState.password);
    onNext();
  };

  return <CreatePassword {...passwordState} onNext={handleCreateAccount} />;
};
