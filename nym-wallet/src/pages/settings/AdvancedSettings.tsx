import React, { useContext, useEffect, useState } from 'react';
import { Box, Button, Divider, Stack, Typography } from '@mui/material';
import { helpLogToggleWindow, logViewerWindowSupported } from '../../requests';
import { AppContext } from '../../context';
import { config } from '../../config';
import SelectValidator from '../../components/Settings/SelectValidator';

const AdvancedSettings = () => {
  const { handleShowTerminal, appEnv } = useContext(AppContext);
  const [logViewerOk, setLogViewerOk] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;
    logViewerWindowSupported()
      .then((ok) => {
        if (!cancelled) setLogViewerOk(ok);
      })
      .catch(() => {
        if (!cancelled) setLogViewerOk(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Box pb={3}>
      {(appEnv?.SHOW_TERMINAL || config.IS_DEV_MODE) && (
        <>
          <Stack direction="row" justifyContent="space-between" padding={3}>
            <Stack direction="column" gap={1}>
              <Typography variant="h6">Terminal</Typography>
              <Typography variant="body2" sx={{ color: 'text.secondary' }}>
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
      {logViewerOk !== null && (
        <>
          <Stack direction="row" justifyContent="space-between" padding={3}>
            <Stack direction="column" gap={1}>
              <Typography variant="h6">Logs</Typography>
              <Typography variant="body2" sx={{ color: 'text.secondary' }}>
                {logViewerOk
                  ? 'Open logs to monitor all actions in the wallet'
                  : 'The in-app log viewer window is not available on Windows. Run the wallet from a terminal or set RUST_LOG to capture logs on stdout.'}
              </Typography>
            </Stack>
            {logViewerOk ? (
              <Box alignSelf="flex-end">
                <Button variant="text" onClick={() => helpLogToggleWindow()}>
                  Open logs
                </Button>
              </Box>
            ) : null}
          </Stack>
          <Divider />
        </>
      )}
      <SelectValidator />
    </Box>
  );
};

export default AdvancedSettings;
