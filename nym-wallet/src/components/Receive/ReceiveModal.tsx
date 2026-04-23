import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { Box, Stack, Typography, alpha, useTheme } from '@mui/material';
import QrCode2Icon from '@mui/icons-material/QrCode2';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { QrCodeReact } from 'src/utils/qrCodeReact';
import { SimpleModal } from '../Modals/SimpleModal';

export const ReceiveModal = ({ onClose }: { onClose: () => void }) => {
  const { clientDetails } = useContext(AppContext);
  const theme = useTheme();

  const isLightMode = theme.palette.mode === 'light';
  const highlightColor = theme.palette.nym.highlight;
  const darkBgColor = theme.palette.background.default;
  const address = clientDetails?.client_address?.trim() ?? '';

  return (
    <SimpleModal
      header="Receive NYM"
      subHeader="Share your address or scan the QR code to receive NYM from another wallet."
      headerCentered
      open
      onClose={onClose}
      okLabel=""
      sx={{
        '& .MuiPaper-root': {
          overflow: 'hidden',
          borderRadius: '20px',
          boxShadow: theme.palette.nym.nymWallet.shadows.strong,
          maxWidth: '480px',
        },
      }}
      subHeaderStyles={{ mb: 0, px: 3, pt: 0.5 }}
    >
      <Stack
        gap={3}
        sx={{
          position: 'relative',
          px: 3,
          pb: 3,
          pt: 0,
        }}
      >
        <Box sx={{ width: '100%' }}>
          <Typography
            variant="caption"
            sx={{
              mb: 1.5,
              color: 'text.secondary',
              fontWeight: 600,
              display: 'block',
              textAlign: 'center',
            }}
          >
            Your address
          </Typography>

          <Box
            sx={{
              p: 2,
              bgcolor: alpha(theme.palette.primary.main, 0.06),
              borderRadius: '12px',
              border: `1px solid ${alpha(theme.palette.primary.main, 0.18)}`,
              position: 'relative',
            }}
          >
            {address ? (
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  justifyContent: 'space-between',
                  gap: 1.5,
                  width: '100%',
                }}
              >
                <Typography
                  component="span"
                  sx={{
                    flex: 1,
                    minWidth: 0,
                    fontSize: '0.9rem',
                    fontFamily: 'monospace',
                    letterSpacing: '0.5px',
                    wordBreak: 'break-all',
                    color: 'text.primary',
                    textAlign: 'left',
                  }}
                >
                  {address}
                </Typography>
                <CopyToClipboard value={address} sx={{ flexShrink: 0, mt: 0.25 }} />
              </Box>
            ) : (
              <Typography variant="body2" color="text.secondary" sx={{ textAlign: 'center' }}>
                No client address available. Sign in again if this persists.
              </Typography>
            )}
          </Box>
        </Box>

        <Box
          sx={{
            position: 'relative',
            width: '100%',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            bgcolor: isLightMode ? alpha(highlightColor, 0.06) : alpha(theme.palette.background.paper, 0.5),
            borderRadius: '16px',
            py: 3,
            px: 2,
            border: `1px solid ${alpha(highlightColor, isLightMode ? 0.2 : 0.12)}`,
          }}
        >
          <Stack direction="row" alignItems="center" gap={1} sx={{ mb: 1.5 }}>
            <QrCode2Icon sx={{ color: 'primary.main', fontSize: 22 }} />
            <Typography variant="subtitle2" fontWeight={600} color="text.primary">
              QR code
            </Typography>
          </Stack>
          <Typography variant="caption" color="text.secondary" sx={{ mb: 2, textAlign: 'center', maxWidth: 320 }}>
            Share this address only with people you trust.
          </Typography>
          <Box
            sx={{
              position: 'relative',
              display: 'flex',
              justifyContent: 'center',
              alignItems: 'center',
              width: '240px',
              height: '240px',
            }}
          >
            <Box
              sx={{
                position: 'absolute',
                width: '100%',
                height: '100%',
                borderRadius: '16px',
                background: `radial-gradient(circle, ${alpha(highlightColor, 0.18)} 0%, transparent 70%)`,
              }}
            />

            <Box
              sx={{
                display: 'flex',
                justifyContent: 'center',
                alignItems: 'center',
                p: 3,
                bgcolor: isLightMode ? 'white' : alpha(darkBgColor, 0.85),
                borderRadius: '16px',
                border: `2px solid ${isLightMode ? highlightColor : alpha(highlightColor, 0.35)}`,
                boxShadow: `0 10px 32px ${alpha(theme.palette.common.black, 0.1)}`,
                transition: 'transform 0.3s ease-in-out, box-shadow 0.3s ease-in-out',
                '&:hover': {
                  transform: 'scale(1.02)',
                  boxShadow: `0 14px 36px ${alpha(theme.palette.common.black, 0.15)}`,
                },
              }}
            >
              {address ? (
                <QrCodeReact
                  renderAs="svg"
                  data-testid="qr-code"
                  value={address}
                  size={200}
                  level="H"
                  includeMargin
                  bgColor={isLightMode ? '#FFFFFF' : theme.palette.background.paper}
                  fgColor={isLightMode ? '#000000' : highlightColor}
                />
              ) : (
                <Typography variant="caption" color="text.secondary">
                  QR unavailable without an address
                </Typography>
              )}
            </Box>
          </Box>

          <Typography
            variant="body2"
            sx={{
              mt: 2.5,
              color: 'text.secondary',
              textAlign: 'center',
              maxWidth: '90%',
              lineHeight: 1.5,
            }}
          >
            Scan this QR code with a compatible wallet to receive NYM tokens
          </Typography>
        </Box>
      </Stack>
    </SimpleModal>
  );
};
