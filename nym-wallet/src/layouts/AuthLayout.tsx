import React, { useContext } from 'react';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
import { Stack, Box, Typography } from '@mui/material';
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
        display: 'grid',
        // Below 2000px: single column (sign-in card centered). At 2000px+: optional left strip of static marketing copy (not navigation).
        gridTemplateColumns: '1fr',
        '@media (min-width: 2000px)': {
          gridTemplateColumns: 'minmax(320px, 420px) minmax(0, 1fr)',
        },
        alignItems: 'stretch',
        bgcolor: 'background.default',
      }}
    >
      <Box
        sx={{
          width: '100%',
          display: { xs: 'none' },
          '@media (min-width: 2000px)': {
            display: 'flex',
          },
          flexDirection: 'column',
          justifyContent: 'space-between',
          px: 6,
          py: 7,
          borderRight: (t) => `1px solid ${t.palette.divider}`,
          background: isDark ? alpha(theme.palette.common.black, 0.2) : theme.palette.nym.nymWallet.background.subtle,
        }}
      >
        <Stack spacing={3}>
          <NymWordmark width={88} fill={wordmarkFill} />
          <Stack spacing={1}>
            <Typography variant="overline" sx={{ color: 'text.secondary', letterSpacing: 1.4 }}>
              Secure wallet access
            </Typography>
            <Typography variant="h3" sx={{ maxWidth: 320, lineHeight: 1.1, color: 'text.primary' }}>
              Access tokens, staking, and node operations in one place.
            </Typography>
            <Typography variant="body1" sx={{ color: 'text.secondary', maxWidth: 340 }}>
              The wallet keeps the same trusted Nym visual language while making key tasks easier to scan and complete.
            </Typography>
          </Stack>
        </Stack>
        <Step />
      </Box>
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          px: { xs: 2, md: 4 },
          py: { xs: 4, md: 6 },
        }}
      >
        <Stack spacing={3} alignItems="center" sx={{ width: '100%', maxWidth: 1080 }}>
          <Box
            sx={{
              display: 'block',
              '@media (min-width: 2000px)': { display: 'none' },
            }}
          >
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
    </Box>
  );
};
