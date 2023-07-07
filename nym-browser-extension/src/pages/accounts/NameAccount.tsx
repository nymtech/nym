import React, { useState } from 'react';
import { TextField } from '@mui/material';
import { Button } from 'src/components';
import { useRegisterContext } from 'src/context/register';
import { TopLogoLayout } from 'src/layouts';
import { useNavigate } from 'react-router-dom';
import { useAppContext } from 'src/context';

export const NameAccount = () => {
  const { accountName, setAccountName } = useRegisterContext();
  const { storage } = useAppContext();
  const navigate = useNavigate();

  const [error, setError] = useState<string>();

  const handleNext = async () => {
    const accountNameExists = await storage?.has_mnemonic(accountName);
    if (accountNameExists) {
      setError('Account name already exists. Please choose another account name');
    } else {
      navigate('/user/accounts/confirm-password');
    }
  };

  return (
    <TopLogoLayout
      title="Name account"
      description="Give your account a unique name"
      Actions={
        <Button fullWidth variant="contained" size="large" onClick={handleNext} disabled={!!error}>
          Next
        </Button>
      }
    >
      <TextField
        fullWidth
        value={accountName}
        onChange={(e) => {
          setError(undefined);
          setAccountName(e.target.value);
        }}
        error={!!error}
        helperText={error}
      />
    </TopLogoLayout>
  );
};
