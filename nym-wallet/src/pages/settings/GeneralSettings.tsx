import React from 'react';
import { Box, Divider, Stack, Typography } from '@mui/material';
import { AppVersion, ThemeSwitcher } from '../../components/Settings';

const GeneralSettings = () => (
  <Box pb={3}>
    <Stack
      direction="row"
      justifyContent="space-between"
      alignItems="flex-start"
      gap={2}
      sx={{ padding: 3, pr: { xs: 3, sm: 4 } }}
    >
      <Stack direction="column" gap={1} sx={{ minWidth: 0 }}>
        <Typography variant="h6">Version</Typography>
        <Typography variant="body2" sx={{ color: 'text.secondary' }}>
          Installed build and update checks
        </Typography>
      </Stack>
      <Box sx={{ flexShrink: 0 }}>
        <AppVersion />
      </Box>
    </Stack>
    <Divider />
    <Stack direction="row" justifyContent="space-between" padding={3}>
      <Stack direction="column" gap={1}>
        <Typography variant="h6">Theme</Typography>
        <Typography variant="body2" sx={{ color: 'text.secondary' }}>
          Select the theme
        </Typography>
      </Stack>
      <Box>
        <ThemeSwitcher />
      </Box>
    </Stack>
  </Box>
);

export default GeneralSettings;
