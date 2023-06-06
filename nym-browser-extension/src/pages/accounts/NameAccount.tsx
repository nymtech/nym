import React from 'react';
import { TextField } from '@mui/material';
import { Button } from 'src/components';
import { useRegisterContext } from 'src/context/register';
import { TopLogoLayout } from 'src/layouts';
import { useNavigate } from 'react-router-dom';

export const NameAccount = () => {
  const { accountName, setAccountName } = useRegisterContext();
  const navigate = useNavigate();

  const handleNext = async () => {
    navigate('/user/accounts/confirm-password');
  };

  return (
    <TopLogoLayout
      title="Name account"
      description="Give your account a unique name"
      Actions={
        <Button fullWidth variant="contained" size="large" onClick={handleNext}>
          Next
        </Button>
      }
    >
      <TextField fullWidth value={accountName} onChange={(e) => setAccountName(e.target.value)} />
    </TopLogoLayout>
  );
};
