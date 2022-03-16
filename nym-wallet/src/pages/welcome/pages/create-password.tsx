import React, { useState } from 'react';
import { Button, FormControl, Grid, IconButton, Stack, TextField } from '@mui/material';
import { VisibilityOff, Visibility } from '@mui/icons-material';
import { TPages } from '../types';
import { Subtitle, Title, PasswordStrength } from '../components';
import { PasswordInput } from '../components/textfields';

export const CreatePassword = ({ page, onPrev, onNext }: { page: TPages; onNext: () => void; onPrev: () => void }) => {
  const [password, setPassword] = useState<string>('');
  const [confirmedPassword, setConfirmedPassword] = useState<string>();
  const [showConfirmedPassword, setShowConfirmedPassword] = useState(false);

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
                <PasswordInput password={password} onUpdatePassword={(pswd) => setPassword(pswd)} />
                <PasswordStrength password={password} />
              </>
              <TextField
                label="Confirm password"
                value={confirmedPassword}
                onChange={(e) => setConfirmedPassword(e.target.value)}
                type={showConfirmedPassword ? 'input' : 'password'}
                InputProps={{
                  endAdornment: (
                    <IconButton onClick={() => setShowConfirmedPassword((show) => !show)}>
                      {showConfirmedPassword ? <VisibilityOff /> : <Visibility />}
                    </IconButton>
                  ),
                }}
              />
              <Button
                size="large"
                variant="contained"
                disabled={password !== confirmedPassword || password.length === 0}
                onClick={onNext}
              >
                Next
              </Button>
              <Button
                size="large"
                onClick={onPrev}
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
