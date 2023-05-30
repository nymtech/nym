import React, { ChangeEvent } from 'react';
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
  const { connectionStatus, serviceProviders, userDefinedSPAddress, setUserDefinedSPAddress } = useClientContext();

  const toggleOnOff = (e: ChangeEvent<HTMLInputElement>) => {
    setUserDefinedSPAddress((current) => ({ ...current, isActive: e.target.checked }));
  };

  const handleSelectFromList = (value: string | null) => {
    setUserDefinedSPAddress((current) => ({ ...current, address: value ?? undefined }));
  };

  const getSPDescription = (spAddress: string) => {
    const match = serviceProviders?.find((sp) => sp.address === spAddress);

    if (match) return match.description;

    return 'N/A';
  };

  const validateInput = (value: string) => {
    setUserDefinedSPAddress((current) => ({ ...current, address: !value.length ? undefined : value }));
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
                  checked={userDefinedSPAddress.isActive}
                  onChange={toggleOnOff}
                  disabled={connectionStatus === ConnectionStatusKind.connected}
                  size="small"
                  sx={{ ml: 1 }}
                />
              }
              label={userDefinedSPAddress.isActive ? 'On' : 'Off'}
            />
            {connectionStatus === ConnectionStatusKind.connected && (
              <FormHelperText sx={{ m: 0, my: 1 }}>This setting is disabled during an active connection</FormHelperText>
            )}
            {userDefinedSPAddress.isActive && serviceProviders && (
              <Autocomplete
                clearOnEscape
                disabled={connectionStatus === 'connected'}
                sx={{ mt: 1 }}
                options={serviceProviders.map((sp) => sp.address)}
                freeSolo
                value={userDefinedSPAddress.address || ''}
                onChange={(e, value) => handleSelectFromList(value)}
                size="small"
                renderInput={(params) => (
                  <TextField
                    autoFocus
                    {...params}
                    placeholder="Service provider"
                    onChange={(e) => validateInput(e.target.value)}
                  />
                )}
                ListboxProps={{ style: { background: 'unset', fontSize: '14px' } }}
              />
            )}
          </FormControl>
          {userDefinedSPAddress.address && userDefinedSPAddress.isActive && (
            <Box sx={{ mt: 2 }}>
              <Typography variant="body2">Name of Service Provider</Typography>
              <Typography variant="body2" sx={{ mt: 0.5 }} color="grey.400">
                {getSPDescription(userDefinedSPAddress.address)}
              </Typography>
            </Box>
          )}
        </Box>
        <AppVersion />
      </Stack>
    </Box>
  );
};
