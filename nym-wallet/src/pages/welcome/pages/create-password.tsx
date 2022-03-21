import React, { useContext, useState } from 'react';
import { Button, FormControl, Grid, Stack } from '@mui/material';
import { TPages } from '../types';
import { Subtitle, Title, PasswordStrength } from '../components';
import { PasswordInput } from '../components/textfields';
import { SignInContext } from '../context';

export const CreatePassword = ({ page, onPrev, onNext }: { page: TPages; onNext: () => void; onPrev: () => void }) => {
  const { password, setPassword } = useContext(SignInContext);
  const [confirmedPassword, setConfirmedPassword] = useState<string>('');
  const [isStrongPassword, setIsStrongPassword] = useState(false);

  const handleOnPrev = () => {
    setPassword('');
    onPrev();
  };

  return (
    <>
      <div id={page} />
      <Title title="Create password" />
      <Subtitle subtitle="Create a strong password. Min 8 characters, at least one capital letter, number and special symbol" />
      <Grid container justifyContent="center">
        <Grid item xs={6}>
          <FormControl fullWidth>
            <Stack spacing={2}>
              <>
                <PasswordInput password={password} onUpdatePassword={(pswd) => setPassword(pswd)} label="Password" />
                <PasswordStrength password={password} onChange={(isStrong) => setIsStrongPassword(isStrong)} />
              </>
              <PasswordInput
                password={confirmedPassword}
                onUpdatePassword={(pswd) => setConfirmedPassword(pswd)}
                label="Confirm password"
              />
              <Button
                size="large"
                variant="contained"
                disabled={password !== confirmedPassword || password.length === 0 || !isStrongPassword}
                onClick={onNext}
              >
                Next
              </Button>
              <Button
                size="large"
                onClick={handleOnPrev}
                sx={{
                  color: 'common.white',
                  border: '1px solid white',
                  '&:hover': { border: '1px solid white', '&:hover': { background: 'none' } },
                }}
              >
                Back
              </Button>
            </Stack>
          </FormControl>
        </Grid>
      </Grid>
    </>
  );
};
