import React from 'react';
import { useCreatePassword } from 'src/hooks/useCreatePassword';
import { useRegisterContext } from 'src/context/register';
import { CreatePasswordTemplate } from 'src/pages/templates/CreatePassword';

export const CreatePasswordOnNewAccount = ({ onNext }: { onNext: () => void }) => {
  const passwordState = useCreatePassword();
  const { setUserPassword } = useRegisterContext();

  const handleCreateAccount = async () => {
    await setUserPassword(passwordState.password);
    onNext();
  };

  return <CreatePasswordTemplate {...passwordState} onNext={handleCreateAccount} />;
};
