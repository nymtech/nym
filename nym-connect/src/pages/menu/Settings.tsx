import React, { ChangeEvent, useState } from 'react';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { Box, FormControl, FormControlLabel, FormHelperText, Switch, Typography } from '@mui/material';
import { useClientContext } from 'src/context/main';
import { ConnectionStatusKind } from 'src/types';

export const Settings = () => {
  const [isActive, setIsActive] = useState(false);
  const [gatewayKey, setGatewayKey] = useState<string>();

  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    setIsActive(e.target.checked);
  };

  const { connectionStatus } = useClientContext();

  return (
    <Box>
      <Typography fontWeight="bold" variant="body2" mb={1}>
        Select your Gateway
      </Typography>
      <Typography color="grey.300" variant="body2" mb={1}>
        Use a gateway of your choice
      </Typography>
      <FormControl fullWidth>
        <FormControlLabel
          control={
            <Switch
              checked={isActive}
              onChange={handleChange}
              disabled={connectionStatus === ConnectionStatusKind.connected}
            />
          }
          label={isActive ? 'On' : 'Off'}
          sx={{ mb: 1 }}
        />
        {connectionStatus === ConnectionStatusKind.connected && (
          <FormHelperText sx={{ m: 0 }}>This option is disabled during an active connection</FormHelperText>
        )}
        {isActive && (
          <IdentityKeyFormField
            size="small"
            placeholder="Gateway identity key"
            onChanged={setGatewayKey}
            initialValue={gatewayKey}
          />
        )}
      </FormControl>
    </Box>
  );
};
