import React, { ChangeEvent, useState } from 'react';
import { Box, FormControl, FormControlLabel, FormHelperText, Stack, Switch, Typography } from '@mui/material';
import { useClientContext } from '../../../context/main';

export const ErrorReporting = () => {
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
            Turn on error reporting and performance monitoring
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
              label={enabled ? 'On' : 'Off'}
            />
            <FormHelperText sx={{ m: 0, my: 2 }}>
              Help Nym developers fix errors, crashes and improve the application by enabling this option. If errors
              occur or if the app crashes, it will automatically send a report. It also tracks various performance
              metrics. We use Sentry.io service to handle this.
            </FormHelperText>
          </FormControl>
          <Typography variant="caption" color="warning.main" fontWeight="bold">
            You must restart the application for the change to take effect.
          </Typography>
        </Box>
      </Stack>
    </Box>
  );
};
