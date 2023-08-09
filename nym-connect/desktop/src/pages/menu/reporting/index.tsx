import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import { Link, List, ListItem, ListItemButton, ListItemText, Stack } from '@mui/material';
import { AppVersion } from 'src/components/AppVersion';

const menuSchema = [
  { title: 'Turn on error reporting', path: 'error-reporting' },
  { title: 'Send us your feedback', path: 'user-feedback' },
];

export const ReportingMenu = () => (
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
    </List>
    <AppVersion />
  </Stack>
);
