import React, { useContext } from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button, Stack, Typography } from '@mui/material';
import QrCode2Icon from '@mui/icons-material/QrCode2';
import SendIcon from '@mui/icons-material/Send';
import TollIcon from '@mui/icons-material/Toll';
import { alpha, useTheme, type Theme } from '@mui/material/styles';
import { AppContext } from '../../context/main';

const iconWrapSx = (theme: Theme) => ({
  width: 40,
  height: 40,
  borderRadius: 2,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  bgcolor: alpha(theme.palette.primary.main, 0.12),
  color: 'primary.main',
  flexShrink: 0,
});

export const OverviewQuickActions = () => {
  const theme = useTheme();
  const navigate = useNavigate();
  const { handleShowReceiveModal, handleShowSendModal } = useContext(AppContext);

  const panelSx = {
    p: 2.5,
    borderRadius: 3,
    border: '1px solid',
    borderColor: 'divider',
    bgcolor: 'background.paper',
    transition: 'border-color 0.2s, box-shadow 0.2s',
    flex: 1,
    minWidth: 0,
    display: 'flex',
    flexDirection: 'column',
    '&:hover': {
      borderColor: alpha(theme.palette.primary.main, 0.45),
      boxShadow: theme.palette.nym.nymWallet.shadows.light,
    },
  } as const;

  const pill = (icon: React.ReactNode, title: string, blurb: string, cta: React.ReactNode) => (
    <Box sx={panelSx}>
      <Stack spacing={1.5} sx={{ height: '100%' }}>
        <Stack direction="row" spacing={1.5} alignItems="flex-start">
          <Box sx={iconWrapSx(theme)}>{icon}</Box>
          <Box sx={{ minWidth: 0 }}>
            <Typography variant="subtitle2" fontWeight={600}>
              {title}
            </Typography>
            <Typography variant="body2" sx={{ color: 'nym.text.muted', lineHeight: 1.35 }}>
              {blurb}
            </Typography>
          </Box>
        </Stack>
        <Box sx={{ mt: 'auto', pt: 0.5 }}>{cta}</Box>
      </Stack>
    </Box>
  );

  return (
    <Stack spacing={1.5} sx={{ width: '100%' }}>
      <Typography variant="caption" sx={{ color: 'nym.text.muted', textTransform: 'uppercase', letterSpacing: 1 }}>
        Quick actions
      </Typography>
      <Stack direction={{ xs: 'column', md: 'row' }} spacing={2} alignItems="stretch">
        {pill(
          <QrCode2Icon fontSize="small" />,
          'Receive',
          'QR and address for incoming NYM',
          <Button variant="contained" fullWidth onClick={handleShowReceiveModal} sx={{ py: 1.25 }}>
            Show QR and address
          </Button>,
        )}
        {pill(
          <SendIcon fontSize="small" />,
          'Send',
          'Transfer NYM to another address',
          <Button variant="outlined" fullWidth onClick={handleShowSendModal} sx={{ py: 1.25 }}>
            Send tokens
          </Button>,
        )}
        {pill(
          <TollIcon fontSize="small" />,
          'Buy',
          'See where to get NYM tokens',
          <Button variant="outlined" fullWidth onClick={() => navigate('/buy')} sx={{ py: 1.25 }}>
            Browse exchanges
          </Button>,
        )}
      </Stack>
    </Stack>
  );
};
