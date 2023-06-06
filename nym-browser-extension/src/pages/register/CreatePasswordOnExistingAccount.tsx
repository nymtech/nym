import React from 'react';
import { useCreatePassword } from 'src/hooks/useCreatePassword';
import { useRegisterContext } from 'src/context/register';
import { CreatePasswordTemplate } from 'src/pages/templates/CreatePassword';

export const CreatePasswordOnExistingAccount = ({ onComplete }: { onComplete: () => void }) => {
  const passwordState = useCreatePassword();
  const { createAccount, userMnemonic } = useRegisterContext();

  const handleOnComplete = async () => {
    await createAccount({ mnemonic: userMnemonic, password: passwordState.password, accName: 'Default account' });
    onComplete();
  };

  return <CreatePasswordTemplate {...passwordState} onNext={handleOnComplete} />;
};
