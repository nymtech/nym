import React from 'react';
import { useCreatePassword } from 'src/hooks/useCreatePassword';
import { CreatePassword } from 'src/components/register/CreatePassword';
import { useRegisterContext } from 'src/context/register';

export const CreatePasswordOnExistingAccount = ({ onComplete }: { onComplete: () => void }) => {
  const passwordState = useCreatePassword();
  const { importExistingAccount } = useRegisterContext();

  const handleOnComplete = async () => {
    await importExistingAccount(passwordState.password);
    onComplete();
  };

  return <CreatePassword {...passwordState} onNext={handleOnComplete} />;
};
