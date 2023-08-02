import React, { ChangeEvent, useState } from 'react';
import { Warning as WarningIcon } from '@mui/icons-material';
import { Box, FormControl, FormControlLabel, FormHelperText, Stack, Switch, Typography } from '@mui/material';
import { useClientContext } from 'src/context/main';

export const MonitoringSettings = () => {
  const { userData, setMonitoring } = useClientContext();
  const [enabled, setEnabled] = useState(userData?.monitoring || false);
  const [loading, setLoading] = useState(false);

  const handleChange = async (e: ChangeEvent<HTMLInputElement>) => {
    setLoading(true);
    setEnabled(e.target.checked);
    await setMonitoring(e.target.checked);
    setLoading(false);
  };

  return (
    <Box height="100%">
      <Stack justifyContent="space-between" height="100%">
        <Box>
          <Typography fontWeight="bold" variant="body2" mb={2}>
            Error reporting and performance monitoring
          </Typography>
          <FormControl fullWidth>
            <FormControlLabel
              control={
                <Switch
                  checked={enabled}
                  onChange={handleChange}
                  disabled={loading}
                  size="small"
                  sx={{ ml: 1, mr: 1 }}
                />
              }
              label="Enable"
            />
            <FormHelperText sx={{ m: 0, my: 2 }}>
              Help Nym developers to fix errors, crashes and improve the application by enabling this option. If errors
              occur or if the app crashes, it will automatically send a report. Also it tracks various performance
              metrics. We use sentry.io service to handle this.
            </FormHelperText>
          </FormControl>
          <Stack direction="row" gap={1} alignItems="center">
            <WarningIcon color="warning" fontSize="small" />
            <Typography variant="caption" color="warning.main">
              You must restart the application for the change to take effect.
            </Typography>
          </Stack>
        </Box>
      </Stack>
    </Box>
  );
};
