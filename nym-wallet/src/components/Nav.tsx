import React, { useContext, useMemo } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { Divider, List, ListItemButton, ListItemIcon, ListItemText, Stack, Typography } from '@mui/material';
import type { Theme } from '@mui/material/styles';
import { alpha } from '@mui/material/styles';
import { AccountBalanceWalletOutlined, Description, Settings, VpnKeyOutlined } from '@mui/icons-material';
import { safeOpenUrl } from 'src/utils/safeOpenUrl';
import { AppContext } from '../context/main';
import { Delegate, Bonding } from '../svg-icons';

const activeNavPrimaryColor = (theme: Theme) =>
  theme.palette.mode === 'dark' ? theme.palette.common.white : theme.palette.grey[900];

export const Nav = () => {
  const location = useLocation();
  const navigate = useNavigate();

  const { isAdminAddress } = useContext(AppContext);

  const routesSchema = useMemo(
    () => [
      {
        label: 'Balance',
        description: 'Portfolio',
        route: '/balance',
        Icon: AccountBalanceWalletOutlined,
        onClick: () => navigate('/balance'),
      },
      {
        label: 'Delegation',
        description: 'Stake and manage rewards',
        route: '/delegation',
        Icon: Delegate,
        onClick: () => navigate('/delegation'),
      },
      {
        label: 'Bonding',
        description: 'Run operator workflows',
        route: '/bonding',
        Icon: Bonding,
        onClick: () => navigate('/bonding'),
      },
      {
        label: 'Docs',
        description: 'Internal wallet notes',
        route: '/docs',
        Icon: Description,
        mode: 'dev',
        onClick: () => navigate('/docs'),
      },
      {
        label: 'Admin',
        description: 'Network management',
        route: '/admin',
        Icon: Settings,
        mode: 'admin',
        onClick: () => navigate('/admin'),
      },
    ],
    [navigate],
  );

  const ecosystemLinks = useMemo(
    () => [
      {
        label: 'NymVPN',
        description: 'Plans and subscribe',
        href: 'https://nym.com/pricing',
        Icon: VpnKeyOutlined,
      },
    ],
    [],
  );

  return (
    <Stack spacing={1.5}>
      <Typography variant="caption" sx={{ color: 'nym.text.muted', textTransform: 'uppercase', letterSpacing: 1 }}>
        Navigation
      </Typography>
      <List
        disablePadding
        sx={{
          width: '100%',
          display: 'flex',
          flexDirection: 'column',
          gap: 1,
        }}
      >
        {routesSchema
          .filter(({ mode }) => {
            if (!mode) {
              return true;
            }
            switch (mode) {
              case 'admin':
                return isAdminAddress;
              case 'dev':
                return isAdminAddress;
              default:
                return false;
            }
          })
          .map(({ Icon, onClick, label, route, description }) => {
            const isActive = route ? location.pathname.startsWith(route) : false;

            return (
              <ListItemButton
                key={label}
                onClick={onClick}
                sx={{
                  px: 2,
                  py: 2,
                  borderRadius: 3,
                  alignItems: 'flex-start',
                  border: (theme) => {
                    if (isActive) {
                      return `1px solid ${theme.palette.primary.main}`;
                    }
                    if (theme.palette.mode === 'dark') {
                      return '1px solid rgba(255,255,255,0.06)';
                    }
                    return `1px solid ${alpha(theme.palette.common.black, 0.08)}`;
                  },
                  backgroundColor: (theme) => {
                    if (isActive) {
                      return `${theme.palette.primary.main}12`;
                    }

                    return theme.palette.mode === 'dark'
                      ? 'rgba(255,255,255,0.02)'
                      : alpha(theme.palette.common.black, 0.02);
                  },
                  '&:hover': {
                    backgroundColor: (theme) =>
                      isActive ? `${theme.palette.primary.main}18` : theme.palette.nym.nymWallet.hover.background,
                  },
                }}
              >
                <ListItemIcon
                  sx={{
                    minWidth: 40,
                    mt: 0.5,
                    color: isActive ? 'primary.main' : 'text.primary',
                  }}
                >
                  <Icon
                    sx={{
                      fontSize: 20,
                      color: 'inherit',
                    }}
                  />
                </ListItemIcon>
                <ListItemText
                  sx={{
                    margin: 0,
                    '& .MuiListItemText-primary': {
                      fontSize: 14,
                      fontWeight: 600,
                      lineHeight: 1.2,
                      color: (theme) => (isActive ? activeNavPrimaryColor(theme) : theme.palette.text.primary),
                    },
                    '& .MuiListItemText-secondary': {
                      mt: 0.5,
                      fontSize: 12,
                      lineHeight: 1.35,
                      color: 'text.secondary',
                    },
                  }}
                  primary={label}
                  secondary={description}
                />
              </ListItemButton>
            );
          })}
        <Divider sx={{ my: 0.5, borderColor: 'divider' }} />
        <Typography variant="caption" sx={{ color: 'nym.text.muted', textTransform: 'uppercase', letterSpacing: 1 }}>
          Ecosystem
        </Typography>
        {ecosystemLinks.map(({ Icon, label, description, href }) => (
          <ListItemButton
            key={label}
            onClick={() => {
              safeOpenUrl(href).catch(() => {
                /* opener unavailable or user cancelled */
              });
            }}
            sx={{
              px: 2,
              py: 2,
              borderRadius: 3,
              alignItems: 'flex-start',
              border: (theme) =>
                theme.palette.mode === 'dark'
                  ? '1px solid rgba(255,255,255,0.06)'
                  : `1px solid ${alpha(theme.palette.common.black, 0.08)}`,
              backgroundColor: (theme) =>
                theme.palette.mode === 'dark' ? 'rgba(255,255,255,0.02)' : alpha(theme.palette.common.black, 0.02),
              '&:hover': {
                backgroundColor: (theme) => theme.palette.nym.nymWallet.hover.background,
              },
            }}
          >
            <ListItemIcon
              sx={{
                minWidth: 40,
                mt: 0.5,
                color: 'primary.main',
              }}
            >
              <Icon sx={{ fontSize: 20, color: 'inherit' }} />
            </ListItemIcon>
            <ListItemText
              sx={{
                margin: 0,
                '& .MuiListItemText-primary': {
                  fontSize: 14,
                  fontWeight: 600,
                  lineHeight: 1.2,
                  color: 'text.primary',
                },
                '& .MuiListItemText-secondary': {
                  mt: 0.5,
                  fontSize: 12,
                  lineHeight: 1.35,
                  color: 'text.secondary',
                },
              }}
              primary={label}
              secondary={description}
            />
          </ListItemButton>
        ))}
      </List>
    </Stack>
  );
};
