import React from 'react';
import { Stack, TextField } from '@mui/material';
import { useLocation, useNavigate } from 'react-router-dom';
import { Button } from 'src/components/ui';
import { CenteredLogoLayout } from 'src/layouts/CenteredLogo';
import { useAppContext } from 'src/context';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { validationSchema } from './validationSchema';

export const Login = () => {
  const { handleUnlockWallet } = useAppContext();
  const navigate = useNavigate();
  const location = useLocation();

  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm({ resolver: zodResolver(validationSchema), defaultValues: { password: '' } });

  const onSubmit = async (data: { password: string }) => {
    try {
      await handleUnlockWallet(data.password);
    } catch (e) {
      setError('password', { message: 'Incorrect password. Please try again.' });
    }
  };

  return (
    <CenteredLogoLayout
      title="Privacy crypto wallet"
      Actions={
        <Stack gap={1} width="100%" justifyContent="flex-end">
          <TextField
            {...register('password')}
            placeholder="Password"
            type="password"
            sx={{ mb: 3 }}
            helperText={errors.password?.message}
            error={!!errors.password}
          />

          <Button
            onClick={handleSubmit(onSubmit)}
            disabled={isSubmitting}
            variant="contained"
            disableElevation
            size="large"
            fullWidth
          >
            {isSubmitting ? 'Loading..' : 'Unlock'}
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
