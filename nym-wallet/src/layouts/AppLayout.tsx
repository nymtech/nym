import React, { useContext } from 'react';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
import { Box, Divider, Stack, Typography } from '@mui/material';
import { alpha } from '@mui/material/styles';
import { AppContext } from 'src/context';
import { AppBar, AppSessionLoadingOverlay, LoadingPage, Nav } from '../components';

export const ApplicationLayout: FCWithChildren = ({ children }) => {
  const { isLoading, loadingPresentation, loadingOverlayTitle, loadingOverlaySubtitle, appVersion } =
    useContext(AppContext);

  return (
    <>
      {isLoading &&
        (loadingPresentation === 'app-overlay' ? (
          <AppSessionLoadingOverlay title={loadingOverlayTitle} subtitle={loadingOverlaySubtitle} />
        ) : (
          <LoadingPage />
        ))}
      <Box
        sx={{
          height: '100vh',
          maxHeight: '100vh',
          width: '100%',
          maxWidth: '100vw',
          overflow: 'hidden',
          display: 'grid',
          gridTemplateColumns: { xs: '1fr', lg: '300px minmax(0, 1fr)' },
          background: (t) =>
            t.palette.mode === 'dark'
              ? `linear-gradient(180deg, ${t.palette.nym.nymWallet.topNav.background} 0%, ${t.palette.background.default} 100%)`
              : t.palette.background.default,
        }}
      >
        <Box
          sx={{
            display: { xs: 'none', lg: 'flex' },
            flexDirection: 'column',
            justifyContent: 'space-between',
            height: '100%',
            minHeight: 0,
            overflowY: 'auto',
            overflowX: 'hidden',
            scrollbarGutter: 'stable',
            px: 3,
            py: 3,
          }}
        >
          <Stack
            spacing={3}
            sx={{
              p: 3,
              borderRadius: 4,
              bgcolor: 'nym.nymWallet.nav.background',
              border: (t) =>
                t.palette.mode === 'light'
                  ? `1px solid ${alpha(t.palette.common.black, 0.08)}`
                  : `1px solid ${t.palette.divider}`,
              boxShadow: (t) => t.palette.nym.nymWallet.shadows.medium,
            }}
          >
            <Stack spacing={1.5}>
              <NymWordmark height={16} />
            </Stack>
            <Divider />
            <Nav />
          </Stack>
          <Stack spacing={1} sx={{ px: 1, pt: 3 }}>
            <Typography
              variant="caption"
              sx={{ color: 'nym.text.muted', textTransform: 'uppercase', letterSpacing: 1 }}
            >
              Nym Wallet
            </Typography>
            {appVersion ? (
              <Typography sx={{ color: 'text.secondary', fontSize: 14 }}>Version {appVersion}</Typography>
            ) : null}
          </Stack>
        </Box>
        <Box
          sx={{
            minWidth: 0,
            maxWidth: '100%',
            height: '100%',
            minHeight: 0,
            overflow: 'hidden',
            display: 'flex',
            flexDirection: 'column',
            px: { xs: 2, md: 3, xl: 4 },
            pb: { xs: 3, md: 4 },
          }}
        >
          <AppBar />
          <Box
            sx={{
              flex: '1 1 auto',
              minHeight: 0,
              overflowY: 'auto',
              overflowX: 'hidden',
              pr: { xs: 0, md: 1 },
              // Avoid horizontal layout shift when scrollbar appears between short/tall routes (e.g. delegation vs bonding).
              scrollbarGutter: 'stable',
            }}
          >
            {children}
          </Box>
        </Box>
      </Box>
    </>
  );
};
