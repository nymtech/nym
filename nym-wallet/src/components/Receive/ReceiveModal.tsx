import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { Box, Stack, SxProps, Typography, alpha, useTheme } from '@mui/material';
import QRCode from 'qrcode.react';
import { ClientAddress } from '@nymproject/react/client-address/ClientAddress';
import { SimpleModal } from '../Modals/SimpleModal';

export const ReceiveModal = ({
  onClose,
  sx,
  backdropProps,
}: {
  onClose: () => void;
  sx?: SxProps;
  backdropProps?: object;
}) => {
  const { clientDetails } = useContext(AppContext);
  const theme = useTheme();

  const isLightMode = theme.palette.mode === 'light';
  const highlightColor = theme.palette.nym.highlight;
  const darkBgColor = theme.palette.background.default;

  return (
    <SimpleModal
      header="Receive"
      open
      onClose={onClose}
      okLabel=""
      sx={{
        ...sx,
        '& .MuiPaper-root': {
          overflow: 'hidden',
          borderRadius: '20px',
          boxShadow: '0 12px 48px rgba(0, 0, 0, 0.15)',
          maxWidth: '480px',
        },
      }}
      backdropProps={backdropProps}
      subHeaderStyles={{ mb: 0, px: 3, pt: 2 }}
    >
      <Stack
        gap={4}
        sx={{
          position: 'relative',
          px: 3,
          pb: 3,
          pt: 1,
        }}
      >
        <Box>
          <Typography
            variant="caption"
            sx={{
              mb: 1.5,
              color: 'text.secondary',
              fontWeight: 600,
              display: 'block',
            }}
          >
            Your address
          </Typography>

          <Box
            sx={{
              p: 2,
              bgcolor: alpha(theme.palette.primary.main, 0.04),
              borderRadius: '12px',
              border: `1px solid ${alpha(theme.palette.primary.main, 0.1)}`,
              position: 'relative',
            }}
          >
            {clientDetails?.client_address && (
              <Box
                sx={{
                  fontSize: '0.9rem',
                  fontFamily: 'monospace',
                  letterSpacing: '0.5px',
                  wordBreak: 'break-all',
                }}
              >
                <ClientAddress address={clientDetails?.client_address} withCopy showEntireAddress />
              </Box>
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
            bgcolor: isLightMode ? alpha(highlightColor, 0.06) : alpha(darkBgColor, 0.7),
            borderRadius: '16px',
            py: 4,
            px: 2,
          }}
        >
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
                background: `radial-gradient(circle, ${alpha(highlightColor, 0.15)} 0%, transparent 70%)`,
              }}
            />

            <Box
              sx={{
                display: 'flex',
                justifyContent: 'center',
                alignItems: 'center',
                p: 3,
                bgcolor: isLightMode ? 'white' : alpha(darkBgColor, 0.7),
                borderRadius: '16px',
                border: `2px solid ${isLightMode ? highlightColor : theme.palette.nym.nymWallet.modal.border}`,
                boxShadow: `0 10px 32px ${alpha(theme.palette.common.black, 0.1)}`,
                transition: 'transform 0.3s ease-in-out, box-shadow 0.3s ease-in-out',
                '&:hover': {
                  transform: 'scale(1.02)',
                  boxShadow: `0 14px 36px ${alpha(theme.palette.common.black, 0.15)}`,
                },
              }}
            >
              {clientDetails && (
                <QRCode
                  data-testid="qr-code"
                  value={clientDetails?.client_address}
                  size={200}
                  level="H"
                  includeMargin
                  bgColor={isLightMode ? '#FFFFFF' : theme.palette.background.paper}
                  fgColor={isLightMode ? '#000000' : highlightColor}
                  imageSettings={{
                    src: '',
                    excavate: true,
                    width: 32,
                    height: 32,
                  }}
                />
              )}
            </Box>
          </Box>

          <Typography
            variant="body2"
            sx={{
              mt: 3,
              color: 'text.secondary',
              textAlign: 'center',
              maxWidth: '80%',
            }}
          >
            Scan this QR code with a compatible wallet to receive NYM tokens
          </Typography>
        </Box>
      </Stack>
    </SimpleModal>
  );
};
