import React from 'react';
import { Apps, HelpOutline, Settings } from '@mui/icons-material';
import { Stack, Link, List, ListItem, ListItemButton, ListItemIcon, ListItemText } from '@mui/material';
import { Link as RouterLink } from 'react-router-dom';
import { AppVersion } from 'src/components/AppVersion';

const menuSchema = [
  { title: 'Supported apps', icon: Apps, path: 'apps' },
  { title: 'How to connect guide', icon: HelpOutline, path: 'guide' },
  { title: 'Settings', icon: Settings, path: 'settings' },
];

export const Menu = () => (
  <Stack justifyContent="space-between" height="100%">
    <List dense disablePadding>
      {menuSchema.map((item) => (
        <Link component={RouterLink} to={item.path} underline="none" color="white" key={item.title}>
          <ListItem disablePadding>
            <ListItemButton>
              <ListItemIcon sx={{ minWidth: 25 }}>
                <item.icon sx={{ fontSize: '12px' }} />
              </ListItemIcon>{' '}
              <ListItemText>{item.title}</ListItemText>
            </ListItemButton>
          </ListItem>
        </Link>
      ))}
    </List>
    <AppVersion />
  </Stack>
);
