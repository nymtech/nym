import React, { ChangeEvent, useState } from 'react';
import {
  Autocomplete,
  Box,
  FormControl,
  FormControlLabel,
  FormHelperText,
  Stack,
  Switch,
  TextField,
  Typography,
} from '@mui/material';
import { AppVersion } from 'src/components/AppVersion';
import { ConnectionStatusKind } from 'src/types';
import { useClientContext } from 'src/context/main';

export const ServiceProviderSettings = () => {
  const [spAddress, setSPAddress] = useState<string>();
  const [isOn, setIsOn] = useState(false);

  const { connectionStatus, serviceProviders } = useClientContext();

  const toggleOnOff = (e: ChangeEvent<HTMLInputElement>) => {
    if (!isOn) setSPAddress(undefined);
    setIsOn(e.target.checked);
  };

  const handleSelectFromList = (value: string | null) => {
    setSPAddress(value ?? undefined);
  };

  return (
    <Box height="100%">
      <Stack justifyContent="space-between" height="100%">
        <Box>
          <Typography fontWeight="bold" variant="body2" mb={1}>
            Select your Service Provider
          </Typography>
          <Typography color="grey.300" variant="body2" mb={2}>
            Pick a service provider from the list or enter your own
          </Typography>
          <FormControl fullWidth>
            <FormControlLabel
              control={
                <Switch
                  checked={isOn}
                  onChange={toggleOnOff}
                  disabled={connectionStatus === ConnectionStatusKind.connected}
                  size="small"
                  sx={{ ml: 1 }}
                />
              }
              label={isOn ? 'On' : 'Off'}
            />
            {connectionStatus === ConnectionStatusKind.connected && (
              <FormHelperText sx={{ m: 0, my: 1 }}>This setting is disabled during an active connection</FormHelperText>
            )}
            {isOn && serviceProviders && (
              <Autocomplete
                clearOnEscape
                sx={{ mt: 1 }}
                options={serviceProviders.map((sp) => `${sp.address.substring(0, 20)}...`)}
                freeSolo
                onChange={(e, value) => handleSelectFromList(value)}
                value={spAddress}
                size="small"
                renderInput={(params) => (
                  <TextField
                    autoFocus
                    {...params}
                    value={spAddress}
                    onChange={(e) => console.log(e.target.value)}
                    placeholder="Service provider"
                  />
                )}
                ListboxProps={{ style: { background: 'unset', fontSize: '14px' } }}
              />
            )}
          </FormControl>
        </Box>
        <AppVersion />
      </Stack>
    </Box>
  );
};
