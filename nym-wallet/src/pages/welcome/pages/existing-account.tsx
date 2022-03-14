/* eslint-disable react/no-unused-prop-types */
import React, { useContext, useState } from 'react';
import { Button, FormControl, Grid, Stack, ToggleButton, ToggleButtonGroup } from '@mui/material';
import { ClientContext } from 'src/context/main';
import { Subtitle } from '../components';
import { TPages } from '../types';
import { MnemonicInput, PasswordInput } from '../components/textfields';

export const ExistingAccount: React.FC<{ page: TPages; onPrev: () => void }> = ({ onPrev }) => {
  const [toggle, setToggle] = useState('mnemonic');
  const { setError } = useContext(ClientContext);

  return (
    <>
      <Subtitle subtitle={`Enter your ${toggle} from existing wallet`} />
      <Grid container justifyContent="center">
        <Grid item xs={6}>
          <Stack spacing={2}>
            <ToggleButtonGroup
              fullWidth
              value={toggle}
              exclusive
              onChange={(e: React.MouseEvent<HTMLElement>, value: string) => {
                setError(undefined);
                setToggle(value);
              }}
            >
              <ToggleButton value="mnemonic">Mnemonic</ToggleButton>
              <ToggleButton value="password">Password</ToggleButton>
            </ToggleButtonGroup>
            <FormControl fullWidth>
              {toggle === 'mnemonic' && <MnemonicInput />}
              {toggle === 'password' && <PasswordInput />}
            </FormControl>
            <Button
              variant="outlined"
              disableElevation
              size="large"
              onClick={() => {
                setError(undefined);
                onPrev();
              }}
              fullWidth
              sx={{ color: 'common.white', border: '1px solid white', '&:hover': { border: '1px solid white' } }}
            >
              Back
            </Button>
          </Stack>
        </Grid>
      </Grid>
    </>
  );
};
