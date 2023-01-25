import React from 'react';
import { Apps, HelpOutline } from '@mui/icons-material';
import { Link, List, ListItem, ListItemButton, ListItemIcon, ListItemText } from '@mui/material';
import { Link as RouterLink } from 'react-router-dom';

const menuSchema = [
  { title: 'Supported apps', icon: Apps, path: 'apps' },
  { title: 'How to connect guide', icon: HelpOutline, path: 'guide' },
];

export const Menu = () => {
  return (
    <List dense disablePadding>
      {menuSchema.map((item) => (
        <Link component={RouterLink} to={item.path} underline="none" color="white">
          <ListItem disablePadding>
            <ListItemButton>
              <ListItemIcon sx={{ minWidth: 25 }}>{<item.icon sx={{ fontSize: '12px' }} />}</ListItemIcon>{' '}
              <ListItemText>{item.title}</ListItemText>
            </ListItemButton>
          </ListItem>
        </Link>
      ))}
    </List>
  );
};
