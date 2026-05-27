import React, { useContext } from 'react';
import { AppBar as MuiAppBar, Box, IconButton, Stack, Toolbar } from '@mui/material';
import { alpha } from '@mui/material/styles';
import { useNavigate } from 'react-router-dom';
import { Logout, SettingsOutlined as SettingsIcon } from '@mui/icons-material';
import { CONTENT_RAIL_MAX_WIDTH_WIDE } from '../layouts/contentRail';
import { AppContext } from '../context/main';
import { NetworkSelector } from './NetworkSelector';
import { MultiAccounts } from './Accounts';

export const AppBar = () => {
  const { logOut } = useContext(AppContext);
  const navigate = useNavigate();

  return (
    <MuiAppBar
      position="sticky"
      sx={{
        boxShadow: 'none',
        bgcolor: 'transparent',
        backgroundImage: 'none',
        pt: { xs: 2, md: 3 },
      }}
    >
      <Toolbar
        disableGutters
        sx={{
          display: 'flex',
          justifyContent: 'center',
          width: '100%',
          minHeight: { xs: 48, sm: 56 },
        }}
      >
        <Box
          sx={{
            width: '100%',
            maxWidth: CONTENT_RAIL_MAX_WIDTH_WIDE,
            mx: 'auto',
            display: 'flex',
            justifyContent: 'flex-end',
            alignItems: 'center',
            borderBottom: (theme) => `1px solid ${alpha(theme.palette.divider, 0.55)}`,
            pb: 1.25,
          }}
        >
          <Stack direction="row" alignItems="center" spacing={1.5} flexWrap="wrap" sx={{ py: 0.5, rowGap: 1 }}>
            <Box sx={{ display: 'flex', alignItems: 'center' }}>
              <MultiAccounts />
            </Box>
            <Box sx={{ display: 'flex', alignItems: 'center' }}>
              <NetworkSelector />
            </Box>
            <Stack direction="row" spacing={0.5} alignItems="center">
              <IconButton size="small" onClick={() => navigate('/settings')} sx={{ color: 'text.primary' }}>
                <SettingsIcon fontSize="small" />
              </IconButton>

              <IconButton
                size="small"
                onClick={async () => {
                  await logOut();
                  navigate('/');
                }}
                sx={{ color: 'text.primary' }}
              >
                <Logout fontSize="small" />
              </IconButton>
            </Stack>
          </Stack>
        </Box>
      </Toolbar>
    </MuiAppBar>
  );
};
