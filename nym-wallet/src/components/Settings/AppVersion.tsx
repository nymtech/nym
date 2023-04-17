import React, { useContext, useEffect, useState } from 'react';
import { Button, Stack, Typography } from '@mui/material';
import { checkUpdate } from '@tauri-apps/api/updater';
import { AppContext } from '../../context';
import { checkVersion } from '../../requests';

const AppVersion = () => {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const { appVersion } = useContext(AppContext);

  const updateCheck = async () => {
    const res = await checkVersion();
    if (res.is_update_available) {
      setUpdateAvailable(true);
    }
  };

  useEffect(() => {
    updateCheck();
  }, [appVersion]);

  const updateHandler = async () => {
    // despite the name, this will spawn an external native window with
    // an embedded "download and install" flow
    checkUpdate();
  };

  return (
    <Stack direction="column" alignItems="flex-end" gap={1}>
      <Stack direction="row" gap={1} alignItems="center">
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          Installed version
        </Typography>
        <Typography>{`Nym Wallet ${appVersion}`}</Typography>
      </Stack>
      {updateAvailable && (
        <Button variant="text" onClick={() => updateHandler()}>
          Update available
        </Button>
      )}
    </Stack>
  );
};

export default AppVersion;
