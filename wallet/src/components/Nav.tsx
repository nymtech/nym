import { useState, useContext } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { List, ListItem, ListItemIcon, ListItemText } from '@mui/material';
import {
  AccountBalanceWalletOutlined,
  ArrowBack,
  ArrowForward,
  Description,
  Settings,
  Toll,
} from '@mui/icons-material';
import { AppContext } from '../context/main';
import { Delegate, Bonding } from '../svg-icons';

export const Nav = () => {
  const location = useLocation();
  const navigate = useNavigate();

  const { isAdminAddress, handleShowSendModal, handleShowReceiveModal } = useContext(AppContext);

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
      Icon: ArrowBack,
      onClick: handleShowReceiveModal,
    },
    {
      label: 'Delegation',
      route: '/delegation',
      Icon: Delegate,
      onClick: () => navigate('/delegation'),
    },
    {
      label: 'Bonding',
      route: '/bonding',
      Icon: Bonding,
      onClick: () => navigate('/bonding'),
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
    {
      label: 'Buy',
      route: '/buy',
      Icon: Toll,
      onClick: () => navigate('/buy'),
    },
  ]);

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'flex-start',
        marginLeft: 12,
        marginRight: 12,
      }}
    >
      <List
        disablePadding
        sx={{
          width: '100%',
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
          .map(({ Icon, onClick, label, route }) => (
            <ListItem
              disableGutters
              key={label}
              onClick={onClick}
              sx={{
                cursor: 'pointer',
                py: 2,
                paddingLeft: 3.5,
                borderRadius: 1,
                '&:hover': { backgroundColor: (theme) => theme.palette.nym.nymWallet.hover.background },
              }}
            >
              <ListItemIcon
                sx={{
                  height: '20px',
                  minWidth: 30,
                  color: location.pathname === route ? 'primary.main' : 'text.primary',
                }}
              >
                <Icon
                  sx={{
                    fontSize: 20,
                  }}
                />
              </ListItemIcon>
              <ListItemText
                sx={{
                  height: '20px',
                  margin: 0,
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
