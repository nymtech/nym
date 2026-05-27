import React, { useContext } from 'react';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
import { Stack, Box } from '@mui/material';
import { alpha, useTheme } from '@mui/material/styles';
import { AppContext } from 'src/context';
import { AppSessionLoadingOverlay, LoadingPage } from 'src/components';
import { Step } from '../pages/auth/components/step';

export const AuthLayout: FCWithChildren = ({ children }) => {
  const { isLoading, loadingPresentation, loadingOverlayTitle, loadingOverlaySubtitle } = useContext(AppContext);
  const theme = useTheme();
  const isDark = theme.palette.mode === 'dark';
  const wordmarkFill = isDark ? '#FFFFFF' : theme.palette.text.primary;

  if (isLoading) {
    if (loadingPresentation === 'app-overlay') {
      return <AppSessionLoadingOverlay title={loadingOverlayTitle} subtitle={loadingOverlaySubtitle} />;
    }
    return <LoadingPage />;
  }

  return (
    <Box
      sx={{
        minHeight: '100vh',
        width: '100%',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        px: { xs: 2, md: 4 },
        py: { xs: 4, md: 6 },
        bgcolor: 'background.default',
      }}
    >
      <Stack spacing={3} alignItems="center" sx={{ width: '100%', maxWidth: 1080 }}>
        <Box sx={{ display: 'block' }}>
          <NymWordmark width={75} fill={wordmarkFill} />
        </Box>
        <Box
          sx={{
            width: '100%',
            maxWidth: 480,
            borderRadius: 4,
            bgcolor: isDark ? alpha(theme.palette.background.paper, 0.92) : 'background.paper',
            border: (t) => `1px solid ${t.palette.divider}`,
            boxShadow: (t) => t.palette.nym.nymWallet.shadows.strong,
            px: { xs: 3, sm: 4 },
            py: { xs: 3, sm: 4 },
            boxSizing: 'border-box',
          }}
        >
          <Stack spacing={2} sx={{ width: '100%', alignItems: 'stretch' }}>
            <Step />
            <Box sx={{ width: '100%' }}>{children}</Box>
          </Stack>
        </Box>
      </Stack>
    </Box>
  );
};
