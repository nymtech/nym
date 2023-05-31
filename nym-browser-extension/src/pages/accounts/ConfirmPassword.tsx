import React from 'react';
import { Button } from 'src/components';
import { useAppContext, useRegisterContext } from 'src/context';
import { TopLogoLayout } from 'src/layouts';
import { PasswordInput } from '@nymproject/react/textfields/Password';
import { useNavigate } from 'react-router-dom';

export const ConfirmPassword = () => {
  const { setAccounts } = useAppContext();
  const { userPassword, setUserPassword, importAccount } = useRegisterContext();

  const navigate = useNavigate();

  const handleOnComplete = async () => {
    const accounts = await importAccount();
    setAccounts(accounts);
    navigate('/user/accounts/import-account/complete');
  };

  return (
    <TopLogoLayout
      title="Confirm password"
      description="Confirm password to import account"
      Actions={
        <Button fullWidth variant="contained" size="large" onClick={handleOnComplete}>
          Confirm
        </Button>
      }
    >
      <PasswordInput value={userPassword} onUpdatePassword={setUserPassword} />
    </TopLogoLayout>
  );
};
