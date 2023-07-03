import React from 'react';
import { FormControlLabel, Checkbox, Stack, Typography, Box } from '@mui/material';
import { TopLogoLayout } from 'src/layouts/TopLogo';
import { PasswordInput } from '@nymproject/react/textfields/Password';
import { PasswordStrength } from '@nymproject/react/password-strength/PasswordStrength';
import { Button } from 'src/components/ui';

type TCreatePassword = {
  canProceed: boolean;
  password: string;
  confirmPassword: string;
  hasReadTerms: boolean;
  setHasReadTerms: (hasReadTerms: boolean) => void;
  setIsSafePassword: (isSafe: boolean) => void;
  setConfirmPassword: (password: string) => void;
  onNext: () => void;
  setPassword: (password: string) => void;
};

export const CreatePasswordTemplate = ({
  canProceed,
  onNext,
  password,
  setPassword,
  confirmPassword,
  setIsSafePassword,
  setConfirmPassword,
  setHasReadTerms,
  hasReadTerms,
}: TCreatePassword) => (
  <TopLogoLayout
    title="Create Password"
    description="Create a strong password - Min 8 characters, at least one capital letter, number and special character"
    Actions={
      <Button fullWidth variant="contained" size="large" disabled={!canProceed} onClick={onNext}>
        Next
      </Button>
    }
  >
    <Stack spacing={1} mb={4}>
      <PasswordInput
        password={password}
        onUpdatePassword={(_password: string) => setPassword(_password)}
        label="Password"
      />

      <PasswordStrength password={password} handleIsSafePassword={(isSafe) => setIsSafePassword(isSafe)} />
    </Stack>

    <Box mb={2}>
      <PasswordInput
        password={confirmPassword}
        onUpdatePassword={(_password: string) => setConfirmPassword(_password)}
        label="Confirm password"
      />
    </Box>

    <FormControlLabel
      label={<Typography variant="caption">I have read and agree with the Terms of use</Typography>}
      control={<Checkbox checked={hasReadTerms} onChange={(_, checked) => setHasReadTerms(checked)} />}
    />
  </TopLogoLayout>
);
