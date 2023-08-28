import React from 'react';
import { TextField } from '@mui/material';
import { Button } from 'src/components';
import { TopLogoLayout } from 'src/layouts';

export const ImportAccountTemplate = ({
  userMnemonic,
  onChangeUserMnemonic,
  onNext,
}: {
  userMnemonic: string;
  onChangeUserMnemonic: (mnemonic: string) => void;
  onNext: () => void;
}) => (
  <TopLogoLayout
    title="Import account"
    description="Type the mnemonic for the account you want to import "
    Actions={
      <Button variant="contained" fullWidth size="large" onClick={onNext}>
        Next
      </Button>
    }
  >
    <TextField
      label="Mnemonic"
      type="password"
      value={userMnemonic}
      onChange={(e) => onChangeUserMnemonic(e.target.value)}
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
