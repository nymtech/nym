import { useContext, useEffect, useState } from 'react';
import { Button, Stack, Typography } from '@mui/material';
import { checkUpdate } from '@tauri-apps/api/updater';
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
      // despite the name, this will spawn an external native window with
      // an embedded "download and update the Wallet" flow
      checkUpdate();
    } catch (e) {
      Console.error(e);
    }
  };

  return (
    <Stack direction="column" alignItems="flex-end" gap={1}>
      <Stack direction="row" gap={1} alignItems="center">
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          Installed version
        </Typography>
        <Typography>{`Nym Wallet v${appVersion}`}</Typography>
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
