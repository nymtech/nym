import React, { useContext, useEffect, useState } from 'react';
import { Button, Stack, Typography } from '@mui/material';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { AppContext } from '../../context';
import { checkVersion } from '../../requests';
import { Console } from '../../utils/console';

const AppVersion = () => {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const { appVersion } = useContext(AppContext);

  const updateCheck = async () => {
    try {
      const res = await checkVersion();
      if (res.is_update_available) {
        setUpdateAvailable(true);
      }
    } catch (e) {
      Console.error(e);
    }
  };

  useEffect(() => {
    updateCheck();
  }, [appVersion]);

  const updateHandler = async () => {
    try {
      const update = await check();
      if (update) {
        await update.downloadAndInstall();
        await relaunch();
      }
    } catch (e) {
      Console.error(e);
    }
  };

  return (
    <Stack direction="column" alignItems="flex-end" gap={1} sx={{ textAlign: 'right' }}>
      <Stack direction="row" gap={1} alignItems="center" flexWrap="wrap" justifyContent="flex-end">
        <Typography variant="caption" sx={{ color: 'text.secondary' }}>
          Installed
        </Typography>
        <Typography fontWeight={600}>{`Nym Wallet v${appVersion}`}</Typography>
      </Stack>
      <Button variant="outlined" size="small" onClick={() => updateCheck()}>
        Check for updates
      </Button>
      {updateAvailable && (
        <Button variant="contained" size="small" onClick={() => updateHandler()}>
          Download update
        </Button>
      )}
    </Stack>
  );
};

export default AppVersion;
