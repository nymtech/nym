import React, { ChangeEvent, useState } from 'react';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { Box, FormControl, FormControlLabel, FormHelperText, Stack, Switch, Typography } from '@mui/material';
import { useClientContext } from 'src/context/main';
import { ConnectionStatusKind } from 'src/types';
import { AppVersion } from 'src/components/AppVersion';

export const Settings = () => {
  const { userDefinedGateway, setUserDefinedGateway } = useClientContext();
  const [gatewayKey, setGatewayKey] = useState<string | undefined>(userDefinedGateway?.gateway);

  const handleIsValidGatewayKey = (isValid: boolean) => {
    let gateway: string | undefined;

    if (isValid) {
      gateway = gatewayKey;
    }

    setUserDefinedGateway((current) => ({ ...current, gateway }));
  };

  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    console.warn('HANERE***');
    setUserDefinedGateway((current) => ({ ...current, isActive: e.target.checked }));
  };

  const { connectionStatus } = useClientContext();

  return (
    <Box height="100%">
      <Stack justifyContent="space-between" height="100%">
        <Box>
          <Typography fontWeight="bold" variant="body2" mb={1} fontSize="14px">
            Select your Gateway
          </Typography>
          <Typography color="grey.300" variant="body2" mb={3}>
            Use a gateway of your choice
          </Typography>
          <FormControl fullWidth>
            <FormControlLabel
              control={
                <Switch
                  checked={userDefinedGateway?.isActive}
                  onChange={handleChange}
                  disabled={connectionStatus === ConnectionStatusKind.connected}
                  size="small"
                  sx={{ ml: 1 }}
                />
              }
              label={userDefinedGateway?.isActive ? 'On' : 'Off'}
            />
            {connectionStatus === ConnectionStatusKind.connected && (
              <FormHelperText sx={{ m: 0, my: 1 }}>This setting is disabled during an active connection</FormHelperText>
            )}
            {userDefinedGateway?.isActive && (
              <IdentityKeyFormField
                size="small"
                placeholder="Gateway identity key"
                onChanged={setGatewayKey}
                initialValue={gatewayKey}
                onValidate={handleIsValidGatewayKey}
                sx={{ mt: 3 }}
                disabled={connectionStatus === 'connected' || !userDefinedGateway?.isActive}
              />
            )}
          </FormControl>
        </Box>
        <Box>
          <Typography variant="body2" mb={4}>
            To find a gateway go to{' '}
            <Typography variant="body2" color="nym.cta">
              explorer.nymtech.net/network-components/gateways
            </Typography>
          </Typography>
          <AppVersion />
        </Box>
      </Stack>
    </Box>
  );
};
