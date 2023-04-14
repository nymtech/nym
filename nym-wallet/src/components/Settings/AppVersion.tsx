import React, { useContext, useEffect, useState } from 'react';
import { Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
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

  return (
    <Stack direction="column" alignItems="flex-end" gap={1}>
      <Stack direction="row" gap={1} alignItems="center">
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          Installed version
        </Typography>
        <Typography>{`Nym Wallet ${appVersion}`}</Typography>
      </Stack>
      {updateAvailable && (
        <Link
          href="https://nymtech.net/download-nym-wallet/"
          target="_blank"
          text="Update available"
          fontWeight={600}
        />
      )}
    </Stack>
  );
};

export default AppVersion;
