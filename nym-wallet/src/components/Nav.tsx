import React, { useContext } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { List, ListItem, ListItemIcon, ListItemText } from '@mui/material';
import { AccountBalanceWalletOutlined, ArrowBack, ArrowForward, Description, Settings } from '@mui/icons-material';
import { AppContext } from '../context/main';
import { Bond, Delegate, Unbond } from '../svg-icons';

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
  const { isAdminAddress } = useContext(AppContext);

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
          .map(({ Icon, route, label }) => (
            <ListItem disableGutters component={Link} to={route} key={label}>
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
                    fontWeight: '600',
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
