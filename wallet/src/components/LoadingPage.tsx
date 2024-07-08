import React from 'react';
import { Box, LinearProgress, Stack } from '@mui/material';
// import { NymWordmark } from '@nymproject/react';
import { AuthTheme } from '@src/theme';

export const LoadingPage = () => (
  <AuthTheme>
    <Box
      sx={{
        position: 'fixed',
        height: '100vh',
        width: '100vw',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        overflow: 'auto',
        bgcolor: 'nym.background.dark',
        zIndex: 2000,
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
          {/* <NymWordmark width={75} fill="white" /> */}
          <Box width="25%">
            <LinearProgress variant="indeterminate" color="primary" />
          </Box>
        </Stack>
      </Box>
    </Box>
  </AuthTheme>
);
