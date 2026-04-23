import React from 'react';
import { Box, CircularProgress, Fade, Stack, Typography } from '@mui/material';
import { alpha, useTheme } from '@mui/material/styles';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';

export type AppSessionLoadingOverlayProps = {
  title: string;
  subtitle?: string;
};

/**
 * In-session full-viewport overlay (blur + card) for account switch and sign-out.
 * Keeps the app theme unlike {@link LoadingPage} which uses the auth splash look.
 */
export const AppSessionLoadingOverlay = ({ title, subtitle }: AppSessionLoadingOverlayProps) => {
  const theme = useTheme();
  const fill = theme.palette.mode === 'dark' ? '#FFFFFF' : theme.palette.text.primary;

  return (
    <Fade in timeout={220}>
      <Box
        role="status"
        aria-live="polite"
        aria-busy="true"
        sx={{
          position: 'fixed',
          inset: 0,
          zIndex: 2000,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          px: 2,
          py: 4,
          bgcolor: (t) => alpha(t.palette.background.default, t.palette.mode === 'dark' ? 0.78 : 0.86),
          backdropFilter: 'blur(14px)',
        }}
      >
        <Box
          sx={{
            width: '100%',
            maxWidth: 400,
            borderRadius: 3,
            border: (t) => `1px solid ${t.palette.divider}`,
            bgcolor: (t) => (t.palette.mode === 'dark' ? alpha(t.palette.background.paper, 0.94) : 'background.paper'),
            boxShadow: (t) => t.palette.nym.nymWallet.shadows.strong,
            px: { xs: 3, sm: 4 },
            py: { xs: 3.5, sm: 4 },
          }}
        >
          <Stack spacing={2.5} alignItems="center" textAlign="center">
            <NymWordmark width={64} fill={fill} />
            <Stack spacing={0.75} alignItems="center">
              <Typography variant="h6" component="p" sx={{ fontWeight: 700, lineHeight: 1.25 }}>
                {title}
              </Typography>
              {subtitle ? (
                <Typography variant="body2" color="text.secondary" sx={{ lineHeight: 1.45, maxWidth: 320 }}>
                  {subtitle}
                </Typography>
              ) : null}
            </Stack>
            <CircularProgress size={40} thickness={4} color="primary" aria-label="Loading" />
          </Stack>
        </Box>
      </Box>
    </Fade>
  );
};
