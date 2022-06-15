import React, { useState, useContext } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { List, ListItem, ListItemIcon, ListItemText } from '@mui/material';
import { AccountBalanceWalletOutlined, ArrowBack, ArrowForward, Description, Settings } from '@mui/icons-material';
import { AppContext } from '../context/main';
import { Bond, Delegate, Unbond, Bonding } from '../svg-icons';

const routesSchema = [
  {
    label: 'Balance',
    route: '/balance',
    Icon: AccountBalanceWalletOutlined,
  },
  {
    label: 'Send',
    route: '/send',
    Icon: ArrowForward,
  },
  {
    label: 'Receive',
    route: '/receive',
    Icon: ArrowBack,
  },
  {
    label: 'Bond',
    route: '/bond',
    Icon: Bond,
  },
  {
    label: 'Bonding',
    route: '/bonding',
    Icon: Bonding,
  },
  {
    label: 'Unbond',
    route: '/unbond',
    Icon: Unbond,
  },
  {
    label: 'Delegation',
    route: '/delegation',
    Icon: Delegate,
  },
  {
    label: 'Docs',
    route: '/docs',
    Icon: Description,
    mode: 'dev',
  },
  {
    label: 'Admin',
    route: '/admin',
    Icon: Settings,
    mode: 'admin',
  },
];

export const Nav = () => {
  const location = useLocation();
  const navigate = useNavigate();

  const { isAdminAddress, handleShowSendModal } = useContext(AppContext);

  const [routesSchema] = useState([
    {
      label: 'Balance',
      route: '/balance',
      Icon: AccountBalanceWalletOutlined,
      onClick: () => navigate('/balance'),
    },
    {
      label: 'Send',
      Icon: ArrowForward,
      onClick: handleShowSendModal,
    },
    {
      label: 'Receive',
      route: '/receive',
      Icon: ArrowBack,
      onClick: () => navigate('/receive'),
    },
    {
      label: 'Bond',
      route: '/bond',
      Icon: Bond,
      onClick: () => navigate('/bond'),
    },
    {
      label: 'Unbond',
      route: '/unbond',
      Icon: Unbond,
      onClick: () => navigate('/unbond'),
    },
    {
      label: 'Delegation',
      route: '/delegation',
      Icon: Delegate,
      onClick: () => navigate('/delegation'),
    },
    {
      label: 'Docs',
      route: '/admin',
      Icon: Description,
      mode: 'dev',
      onClick: () => navigate('/docs'),
    },
    {
      label: 'Admin',
      route: '/admin',
      Icon: Settings,
      mode: 'admin',
      onClick: () => navigate('/admin'),
    },
  ]);

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'flex-start',
      }}
    >
      <List disablePadding>
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
          .map(({ Icon, onClick, label, route }) => (
            <ListItem disableGutters key={label} onClick={onClick} sx={{ cursor: 'pointer' }}>
              <ListItemIcon
                sx={{
                  minWidth: 30,
                  color: location.pathname === route ? 'primary.main' : 'text.primary',
                }}
              >
                <Icon sx={{ fontSize: 20 }} />
              </ListItemIcon>
              <ListItemText
                sx={{
                  color: location.pathname === route ? 'primary.main' : 'text.primary',
                  '& .MuiListItemText-primary': {
                    fontSize: 14,
                    fontWeight: (theme) => (theme.palette.mode === 'light' ? 600 : 500),
                  },
                }}
                primary={label}
              />
            </ListItem>
          ))}
      </List>
    </div>
  );
};
