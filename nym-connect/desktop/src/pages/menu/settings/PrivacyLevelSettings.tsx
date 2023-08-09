import React, { ChangeEvent, useState } from 'react';
import * as Sentry from '@sentry/react';
import { Box, FormControl, FormControlLabel, FormHelperText, Stack, Switch, Typography } from '@mui/material';
import { useClientContext } from 'src/context/main';

export const PrivacyLevelSettings = () => {
  const { userData, setPrivacyLevel } = useClientContext();
  const [speedBoost, setSpeedBoost] = useState(userData?.privacy_level !== 'High');
  const [loading, setLoading] = useState(false);

  const handleChange = async (e: ChangeEvent<HTMLInputElement>) => {
    setLoading(true);
    setSpeedBoost(e.target.checked);
    Sentry.captureMessage(`privacy level switched to ${e.target.checked ? 'Medium' : 'High'}`, 'info');
    await setPrivacyLevel(e.target.checked ? 'Medium' : 'High');
    setLoading(false);
  };

  return (
    <Box height="100%">
      <Stack justifyContent="space-between" height="100%">
        <Box>
          <Typography fontWeight="bold" variant="body2" mb={2}>
            Speed boost
          </Typography>
          <FormControl fullWidth>
            <FormControlLabel
              control={
                <Switch
                  checked={speedBoost}
                  onChange={handleChange}
                  disabled={loading}
                  size="small"
                  sx={{ ml: 1, mr: 1 }}
                />
              }
              label="Enable"
            />
            <FormHelperText sx={{ m: 0, my: 2 }}>
              By activating this option, the connection speed will be relatively faster in exchange for relaxing some
              privacy protections
            </FormHelperText>
          </FormControl>
        </Box>
      </Stack>
    </Box>
  );
};
