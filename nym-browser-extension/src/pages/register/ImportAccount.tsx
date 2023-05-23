import React from 'react';
import { TextField } from '@mui/material';
import { useLocation, useNavigate } from 'react-router-dom';
import { Button } from 'src/components';
import { TopLogoLayout } from 'src/layouts';
import { useRegisterContext } from 'src/context/register';

export const ImportAccount = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const { setUserMnemonic, userMnemonic } = useRegisterContext();

  const handleNext = async () => {
    navigate(`${location.pathname}/create-password`);
  };

  return (
    <TopLogoLayout
      title="Import account"
      description="Type the mnemonic for the account you want to import "
      Actions={
        <Button variant="contained" fullWidth size="large" onClick={handleNext}>
          Next
        </Button>
      }
    >
      <TextField
        label="Mnemonic"
        type="password"
        value={userMnemonic}
        onChange={(e) => setUserMnemonic(e.target.value)}
        multiline
        autoFocus={false}
        fullWidth
        inputProps={{
          style: {
            height: '160px',
          },
        }}
        InputLabelProps={{ shrink: true }}
        sx={{
          'input::-webkit-textfield-decoration-container': {
            alignItems: 'start',
          },
        }}
      />
    </TopLogoLayout>
  );
};
