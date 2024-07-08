import React, { useContext } from 'react';
import { Box, Button, Divider, Stack, Typography } from '@mui/material';
import { helpLogToggleWindow } from '../../requests';
import { AppContext } from '../../context';
import { config } from '../../config';
import SelectValidator from '../../components/Settings/SelectValidator';

const AdvancedSettings = () => {
  const { handleShowTerminal, appEnv } = useContext(AppContext);

  return (
    <Box pb={3}>
      {(appEnv?.SHOW_TERMINAL || config.IS_DEV_MODE) && (
        <>
          <Stack direction="row" justifyContent="space-between" padding={3}>
            <Stack direction="column" gap={1}>
              <Typography variant="h6">Terminal</Typography>
              <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
                Open the terminal (dev mode)
              </Typography>
            </Stack>
            <Box alignSelf="flex-end">
              <Button variant="text" onClick={() => handleShowTerminal()}>
                Open terminal
              </Button>
            </Box>
          </Stack>
          <Divider />
        </>
      )}
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
      <Divider />
      <SelectValidator />
    </Box>
  );
};

export default AdvancedSettings;
