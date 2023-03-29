import React from 'react';
import { Box, Button, Stack, Typography } from '@mui/material';
import { helpLogToggleWindow } from '../../requests';

const AdvancedSettings = () => (
  <Box pb={3}>
    <Stack direction="row" justifyContent="space-between" padding={3}>
      <Stack direction="column" gap={1}>
        <Typography variant="h6">Logs</Typography>
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          Open logs to monitor all actions in the wallet
        </Typography>
      </Stack>
      <Box alignSelf="flex-end">
        <Button variant="text" onClick={() => helpLogToggleWindow()}>
          Open logs
        </Button>
      </Box>
    </Stack>
  </Box>
);

export default AdvancedSettings;
