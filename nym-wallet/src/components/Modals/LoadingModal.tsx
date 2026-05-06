import React from 'react';
import { BackdropProps, Box, CircularProgress, Fade, Modal, Stack, Typography, SxProps } from '@mui/material';
import { alpha, useTheme, type Theme } from '@mui/material/styles';

export const LoadingModal: FCWithChildren<{
  text?: string;
  sx?: SxProps;
  backdropProps?: Partial<BackdropProps>;
}> = ({ sx, backdropProps, text = 'Just a moment' }) => {
  const theme = useTheme();
  const { sx: backdropSx, ...backdropRest } = backdropProps || {};

  const extraBackdropSx: SxProps<Theme>[] = [];
  if (Array.isArray(backdropSx)) {
    extraBackdropSx.push(...(backdropSx as readonly SxProps<Theme>[]));
  } else if (backdropSx) {
    extraBackdropSx.push(backdropSx as SxProps<Theme>);
  }

  // MUI merges `sx` arrays at runtime; a plain TS array is not inferred as SxProps.
  const backdropSxMerged: SxProps<Theme> = [
    {
      backdropFilter: 'blur(12px)',
      backgroundColor: alpha(theme.palette.common.black, theme.palette.mode === 'dark' ? 0.5 : 0.34),
    },
    ...extraBackdropSx,
  ] as SxProps<Theme>;

  return (
    <Modal
      open
      closeAfterTransition
      disableAutoFocus
      BackdropProps={{
        ...backdropRest,
        sx: backdropSxMerged,
      }}
    >
      <Fade in timeout={200}>
        <Box
          role="status"
          aria-live="polite"
          aria-busy="true"
          tabIndex={-1}
          sx={{
            position: 'absolute',
            top: '50%',
            left: '50%',
            transform: 'translate(-50%, -50%)',
            width: 'min(400px, calc(100% - 32px))',
            borderRadius: 3,
            border: (t) => `1px solid ${alpha(t.palette.divider, theme.palette.mode === 'dark' ? 0.55 : 1)}`,
            bgcolor: (t) =>
              theme.palette.mode === 'dark' ? alpha(t.palette.background.paper, 0.96) : t.palette.background.paper,
            boxShadow: (t) => t.palette.nym.nymWallet.shadows.strong,
            px: { xs: 3, sm: 4 },
            py: { xs: 3.5, sm: 4.25 },
            outline: 'none',
            ...sx,
          }}
        >
          <Stack spacing={2.25} alignItems="center" textAlign="center">
            <CircularProgress size={40} thickness={4} color="primary" aria-label="Loading" />
            <Typography
              variant="body1"
              sx={{
                color: 'text.primary',
                fontWeight: 600,
                letterSpacing: 0.02,
                lineHeight: 1.45,
                fontSize: { xs: '0.95rem', sm: '1rem' },
              }}
            >
              {text}
            </Typography>
          </Stack>
        </Box>
      </Fade>
    </Modal>
  );
};
