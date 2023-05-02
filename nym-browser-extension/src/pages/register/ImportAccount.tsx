import React from 'react';
import { TextField } from '@mui/material';
import { useLocation, useNavigate } from 'react-router-dom';
import { Button } from 'src/components';
import { TopLogoLayout } from 'src/layouts';

export const ImportAccount = () => {
  const navigate = useNavigate();
  const location = useLocation();
  return (
    <TopLogoLayout
      title="Import account"
      description="Type the mnemonic for the account you want to import "
      Actions={
        <Button
          variant="contained"
          fullWidth
          size="large"
          onClick={() => navigate(`${location.pathname}/create-password`)}
        >
          Next
        </Button>
      }
    >
      <TextField
        label="Mnemonic"
        type="input"
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
