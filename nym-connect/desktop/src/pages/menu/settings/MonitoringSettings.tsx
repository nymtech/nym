import React, { ChangeEvent, useState } from 'react';
import { Box, FormControl, FormControlLabel, FormHelperText, Stack, Switch, Typography } from '@mui/material';
import { useClientContext } from 'src/context/main';

export const MonitoringSettings = () => {
  const { monitoringEnabled, setMonitoring } = useClientContext();
  const [enabled, setEnabled] = useState(monitoringEnabled);
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
            Errors reporting and performance monitoring
          </Typography>
          <FormControl fullWidth>
            <FormControlLabel
              control={
                <Switch checked={enabled} onChange={handleChange} disabled={loading} size="small" sx={{ ml: 1 }} />
              }
              label="Enable"
            />
            <FormHelperText sx={{ m: 0, my: 2 }}>
              Help developers to fix errors and improve the application.
            </FormHelperText>
          </FormControl>
          <Typography variant="caption" color={(t) => t.palette.nym.warning}>
            ⚠ You must restart the application for the change to take effect.
          </Typography>
        </Box>
      </Stack>
    </Box>
  );
};
