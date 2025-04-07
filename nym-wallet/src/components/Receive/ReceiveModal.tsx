import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { Box, Stack, SxProps, Typography, alpha, useTheme } from '@mui/material';
import QRCode from 'qrcode.react';
import { ClientAddress } from '@nymproject/react/client-address/ClientAddress';
import { ModalListItem } from '../Modals/ModalListItem';
import { SimpleModal } from '../Modals/SimpleModal';
import { ArrowDownward } from '@mui/icons-material';

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
          borderRadius: '16px',
          boxShadow: '0 8px 32px rgba(0, 0, 0, 0.12)'
        }
      }}
      backdropProps={backdropProps}
      subHeaderStyles={{ mb: 0 }}
    >
      <Stack gap={3} sx={{ position: 'relative', pt: 1, pb: 0 }}>
        <Box sx={{ px: 2 }}>
          <Typography variant="subtitle1" sx={{ mb: 1, color: 'text.secondary', fontWeight: 500 }}>
            Your address
          </Typography>
          
          <Box 
            sx={{ 
              p: 2, 
              bgcolor: alpha(theme.palette.primary.main, 0.04),
              borderRadius: '12px',
              border: `1px solid ${alpha(theme.palette.primary.main, 0.1)}`
            }}
          >
            {clientDetails?.client_address && (
              <Box sx={{
                  fontSize: '0.9rem',
                  fontFamily: 'monospace',
                  letterSpacing: '0.5px'
                }}>
                <ClientAddress 
                  address={clientDetails?.client_address} 
                  withCopy 
                  showEntireAddress
                />
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
            pt: 1,
            pb: 5,
            mt: 2,
            bgcolor: isLightMode 
              ? alpha(highlightColor, 0.06)
              : alpha(darkBgColor, 0.7),
            borderRadius: '0 0 16px 16px',
          }}
        >
          <Box
            sx={{
              position: 'relative',
              display: 'flex',
              justifyContent: 'center',
              alignItems: 'center',
              width: '200px',
              height: '200px',
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
                boxShadow: `0 8px 24px ${alpha(theme.palette.common.black, 0.1)}`,
                transition: 'transform 0.3s ease-in-out, box-shadow 0.3s ease-in-out',
                '&:hover': {
                  transform: 'scale(1.02)',
                  boxShadow: `0 12px 28px ${alpha(theme.palette.common.black, 0.15)}`,
                }
              }}
            >
              {clientDetails && (
                <QRCode 
                  data-testid="qr-code" 
                  value={clientDetails?.client_address}
                  size={160}
                  level="H"
                  includeMargin={true}
                  bgColor={isLightMode ? "#FFFFFF" : theme.palette.background.paper}
                  fgColor={isLightMode ? "#000000" : highlightColor}
                  imageSettings={{
                    src: "",
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
              maxWidth: '80%'
            }}
          >
            Scan this QR code with a compatible wallet to receive NYM tokens
          </Typography>
        </Box>
      </Stack>
    </SimpleModal>
  );
};