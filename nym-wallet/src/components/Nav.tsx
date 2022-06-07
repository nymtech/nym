import React, { useContext, useEffect } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { List, ListItem, ListItemIcon, ListItemText } from '@mui/material';
import { AccountBalanceWalletOutlined, ArrowBack, ArrowForward, Description, Settings } from '@mui/icons-material';
import { AppContext } from '../context/main';
import { Bond, Delegate, Unbond, Undelegate } from '../svg-icons';

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
];

export const Nav = () => {
  const { isAdminAddress, handleShowAdmin } = useContext(AppContext);
  const location = useLocation();

  useEffect(() => {
    if (isAdminAddress) {
      routesSchema.push({
        label: 'Docs',
        route: '/docs',
        Icon: Description,
      });
    }
  }, [isAdminAddress]);

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'flex-start',
      }}
    >
      <List disablePadding>
        {routesSchema.map(({ Icon, route, label }) => (
          <ListItem disableGutters component={Link} to={route} key={label}>
            <ListItemIcon
              sx={{
                minWidth: 30,
                color: location.pathname === route ? 'primary.main' : 'common.white',
              }}
            >
              <Icon sx={{ fontSize: 20 }} />
            </ListItemIcon>
            <ListItemText
              sx={{
                color: location.pathname === route ? 'primary.main' : 'common.white',
              }}
              primary={label}
            />
          </ListItem>
        ))}
        {isAdminAddress && (
          <ListItem disableGutters onClick={handleShowAdmin}>
            <ListItemIcon
              sx={{
                minWidth: 30,
              }}
            >
              <Settings sx={{ fontSize: 20, color: 'white' }} />
            </ListItemIcon>
            <ListItemText primary="Admin" sx={{ color: 'common.white' }} />
          </ListItem>
        )}
      </List>
    </div>
  );
};
