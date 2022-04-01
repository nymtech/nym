import React from 'react';
import { Stack, Box } from '@mui/material';
import { NymWordmark } from '@nymproject/react';
import { Step } from './step';

export const PageLayout: React.FC = ({ children }) => {
  return (
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
          <NymWordmark width={75} />
          <Step />
          {children}
        </Stack>
      </Box>
    </Box>
  );
};
