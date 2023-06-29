import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import { Link, List, ListItem, ListItemButton, ListItemText, Stack } from '@mui/material';
import { AppVersion } from 'src/components/AppVersion';
import { toggleLogViewer } from 'src/utils';

const menuSchema = [
  { title: 'Select your gateway', path: 'gateway' },
  { title: 'Select a service provider', path: 'service-provider' },
];

export const SettingsMenu = () => (
  <Stack justifyContent="space-between" height="100%">
    <List dense disablePadding>
      {menuSchema.map((item) => (
        <Link component={RouterLink} to={item.path} underline="none" color="white" key={item.title}>
          <ListItem disablePadding>
            <ListItemButton>
              <ListItemText>{item.title}</ListItemText>
            </ListItemButton>
          </ListItem>
        </Link>
      ))}
      <ListItem disablePadding>
        <ListItemButton onClick={toggleLogViewer}>
          <ListItemText>Logs</ListItemText>
        </ListItemButton>
      </ListItem>
    </List>
    <AppVersion />
  </Stack>
);
