import React, { useContext } from 'react';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
import { Stack, Box } from '@mui/material';
import { AppContext } from 'src/context';
import { LoadingPage } from 'src/components';
import { Step } from '../pages/auth/components/step';

export const AuthLayout: React.FC = ({ children }) => {
  const { isLoading } = useContext(AppContext);

  return isLoading ? (
    <LoadingPage />
  ) : (
    <Box
      sx={{
        height: '100vh',
        width: '100vw',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        overflow: 'auto',
        bgcolor: 'nym.background.dark',
      }}
    >
      <Box
        sx={{
          width: '100%',
          display: 'flex',
          justifyContent: 'center',
          margin: 'auto',
        }}
      >
        <Stack spacing={3} alignItems="center" sx={{ width: 1080 }}>
          <NymWordmark width={75} fill="#FFFFFF" />
          <Step />
          {children}
        </Stack>
      </Box>
    </Box>
  );
};
