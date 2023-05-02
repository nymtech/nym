import React, { useState } from 'react';
import { Stack, TextField } from '@mui/material';
import { useLocation, useNavigate } from 'react-router-dom';
import { Button } from 'src/components/ui';
import { CenteredLogoLayout } from 'src/layouts/CenteredLogo';
import { useAppContext } from 'src/context';

export const Login = () => {
  const [password, setPassword] = useState('');
  const { handleUnlockWallet } = useAppContext();
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <CenteredLogoLayout
      title="Privacy crypto wallet"
      Actions={
        <Stack gap={1} width="100%" justifyContent="flex-end">
          <TextField
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="Password"
            type="password"
            sx={{ mb: 3 }}
          />
          <Button
            onClick={() => handleUnlockWallet(password)}
            variant="contained"
            disableElevation
            size="large"
            fullWidth
          >
            Unlock
          </Button>
          <Button
            variant="outlined"
            disableElevation
            size="large"
            fullWidth
            color="primary"
            onClick={() => navigate(`${location.pathname}/forgot-password`)}
          >
            Forgot password?
          </Button>
        </Stack>
      }
    />
  );
};
