import React, { useContext, useEffect, useState } from 'react';
import { Stack, Typography } from '@mui/material';
import { checkUpdate } from '@tauri-apps/api/updater';
import { AppContext } from '../../context';

const AppVersion = () => {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const { appVersion } = useContext(AppContext);

  const updateCheck = async () => {
    const update = await checkUpdate();
    if (update.shouldUpdate && update.manifest) {
      setUpdateAvailable(true);
    } else {
      setUpdateAvailable(false);
    }
  };

  useEffect(() => {
    updateCheck();
  }, [appVersion]);

  return (
    <Stack direction="column" alignItems="flex-end" gap={1}>
      <Stack direction="row" gap={1} alignItems="center">
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          Installed version
        </Typography>
        <Typography>{`Nym Wallet ${appVersion}`}</Typography>
      </Stack>
      {updateAvailable && (
        <Typography color="primary" fontWeight={600}>
          Update available
        </Typography>
      )}
    </Stack>
  );
};

export default AppVersion;
