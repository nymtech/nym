import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import { Link, List, ListItem, ListItemButton, ListItemText, Stack } from '@mui/material';
import { AppVersion } from 'src/components/AppVersion';
import { toggle_log_viewer } from 'src/utils';

const menuSchema = [{ title: 'Select your gateway', path: 'gateway' }];

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
        <ListItemButton onClick={toggle_log_viewer}>
          <ListItemText>Logs</ListItemText>
        </ListItemButton>
      </ListItem>
    </List>
    <AppVersion />
  </Stack>
);
