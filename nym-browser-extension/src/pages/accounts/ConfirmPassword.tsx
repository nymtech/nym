import React, { useState } from 'react';
import { PasswordInput } from '@nymproject/react/textfields/Password';
import { Button } from 'src/components';
import { useAppContext, useRegisterContext } from 'src/context';
import { TopLogoLayout } from 'src/layouts';
import { useNavigate } from 'react-router-dom';

export const ConfirmPassword = () => {
  const { setAccounts } = useAppContext();
  const { userPassword, setUserPassword, importAccount } = useRegisterContext();
  const [error, setError] = useState<string>();

  const navigate = useNavigate();

  const handleOnComplete = async () => {
    try {
      const accounts = await importAccount();
      setAccounts(accounts);
      navigate('/user/accounts/complete');
    } catch (e) {
      setError('Incorrect password. Please try again');
    }
  };

  const onChange = (password: string) => {
    setError(undefined);
    setUserPassword(password);
  };

  return (
    <TopLogoLayout
      title="Confirm password"
      description="Confirm password to import account"
      Actions={
        <Button fullWidth variant="contained" size="large" onClick={handleOnComplete} disabled={!!error}>
          Confirm
        </Button>
      }
    >
      <PasswordInput value={userPassword} onUpdatePassword={onChange} error={error} />
    </TopLogoLayout>
  );
};
