import * as React from 'react';
import Box from '@mui/material/Box';
import Drawer from '@mui/material/Drawer';
import List from '@mui/material/List';
import ListItem from '@mui/material/ListItem';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import { AccountBalanceWalletRounded, ArrowDownwardRounded } from '@mui/icons-material';
import { Link } from 'react-router-dom';

const menuSchema = [
  {
    title: 'Balance',
    Icon: <AccountBalanceWalletRounded />,
    path: '/user/balance',
  },
  {
    title: 'Send',
    Icon: <ArrowDownwardRounded />,
    path: '/user/send',
  },
];

export const MenuDrawer = ({ open, onClose }: { open: boolean; onClose: () => void }) => {
  const list = () => (
    <Box sx={{ width: 250 }} role="presentation" onClick={() => {}}>
      <List>
        {menuSchema.map(({ title, Icon, path }) => (
          <Link to={path} style={{ textDecoration: 'none', color: 'unset' }} key={title}>
            <ListItem disablePadding>
              <ListItemButton>
                <ListItemIcon>{Icon}</ListItemIcon>
                <ListItemText primary={title} />
              </ListItemButton>
            </ListItem>
          </Link>
        ))}
      </List>
    </Box>
  );

  return (
    <div>
      <Drawer anchor="left" open={open} onClose={onClose}>
        {list()}
      </Drawer>
    </div>
  );
};
