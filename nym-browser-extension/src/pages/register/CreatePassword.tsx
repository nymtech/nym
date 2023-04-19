import React, { useState } from 'react';
import { FormControlLabel, Checkbox, Stack, Typography, Box } from '@mui/material';
import { TopLogoLayout } from 'src/layouts/TopLogo';
import { PasswordInput } from '@nymproject/react/textfields/Password';
import { PasswordStrength } from '@nymproject/react/password-strength/PasswordStrength';
import { Button } from 'src/components/ui';
import { useNavigate } from 'react-router-dom';

export const CreatePassword = () => {
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isSafePassword, setIsSafePassword] = useState(false);
  const [hasReadTerms, setHasReadTerms] = useState(false);

  const canProceed = isSafePassword && hasReadTerms && password === confirmPassword;

  const navigate = useNavigate();
  return (
    <TopLogoLayout
      title="Create Password"
      description="Create strong password, min 8 characters, at least one capital letter, number and special character"
      Actions={
        <Button
          fullWidth
          variant="contained"
          size="large"
          disabled={!canProceed}
          onClick={() => navigate('/register/complete')}
        >
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
};
