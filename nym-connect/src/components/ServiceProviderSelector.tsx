import React, { useEffect, useMemo } from 'react';
import { Box, CircularProgress, Stack, TextField, Tooltip, Typography, MenuItem, ListItemIcon } from '@mui/material';
import { ServiceProvider, Service, Services } from '../types/directory';
import { useTauriEvents } from '../utils';

type ServiceWithRandomSp = {
  id: string;
  description: string;
  sp: ServiceProvider;
};

const defaultServiceValue = { id: '', description: '', items: [] };

export const ServiceProviderSelector: FCWithChildren<{
  onChange?: (serviceProvider?: ServiceProvider) => void;
  services?: Services;
  currentSp?: ServiceProvider;
}> = ({ services, currentSp, onChange }) => {
  const [service, setService] = React.useState<Service>(defaultServiceValue);
  const [serviceProvider, setServiceProvider] = React.useState<ServiceProvider | undefined>(currentSp);
  const [resetTrigger, setResetTrigger] = React.useState(new Date().toISOString());

  const handleSelectSp = (newServiceProvider?: ServiceProvider) => {
    if (newServiceProvider && newServiceProvider !== currentSp) {
      setServiceProvider(newServiceProvider);
      onChange?.(newServiceProvider);
    }
  };

  // when the user clears local storage, reset the selector
  useTauriEvents('help://clear-storage', () => {
    setService(defaultServiceValue);
    setServiceProvider(undefined);
    onChange?.(undefined);
    setResetTrigger(new Date().toISOString());
  });

  useEffect(() => {
    if (!serviceProvider && currentSp) {
      setServiceProvider(currentSp);
    }
  }, [currentSp]);

  useEffect(() => {
    if (services && serviceProvider) {
      // retrieve the service corresponding to this service provider

      const match = services.find((s) =>
        s.items.some(
          ({ id, address, gateway }) =>
            id === serviceProvider.id && address === serviceProvider.address && gateway === serviceProvider.gateway,
        ),
      );

      if (match) {
        setService(match);
      }
    }
  }, [serviceProvider, services]);

  if (!services) {
    return (
      <Box display="flex" alignItems="center" justifyContent="center" sx={{ my: 3 }}>
        <Typography fontSize={14} fontWeight={700} color={(theme) => theme.palette.common.white}>
          <CircularProgress size={14} sx={{ mr: 1 }} color="inherit" />
          Loading services...
        </Typography>
      </Box>
    );
  }

  const servicesWithRandomSp: ServiceWithRandomSp[] = useMemo(
    () =>
      services.map(({ id, items, description }) => ({
        id,
        description,
        sp: items[Math.floor(Math.random() * items.length)],
      })),
    [services, resetTrigger],
  );

  if (!service) return null;

  return (
    <Box display="flex" alignItems="center" justifyContent="space-between" sx={{ my: 3 }}>
      <TextField
        variant="filled"
        select
        fullWidth
        value={service.description}
        label="Select a service"
        InputLabelProps={{
          sx: {
            color: 'grey.500',
            '&.Mui-focused': {
              color: 'grey.500',
            },
          },
        }}
        SelectProps={{
          MenuProps: {
            PaperProps: {
              sx: {
                background: '#383C41',
                borderTopLeftRadius: 0,
                borderTopRightRadius: 0,
                '&& .Mui-selected': {
                  backgroundColor: '#FFFFFF33',
                },
                '&& .Mui-focusVisible': {
                  backgroundColor: '#FFFFFF33',
                },
              },
            },
          },
        }}
      >
        {servicesWithRandomSp.map(({ id, description, sp }) => (
          <MenuItem key={id} value={description} onClick={() => handleSelectSp(sp)}>
            <Tooltip
              title={
                <Stack direction="column">
                  <Typography fontSize="inherit">
                    <code>{sp.id}</code>
                  </Typography>
                  <Typography fontSize="inherit" fontWeight={700}>
                    {sp.description}
                  </Typography>
                  <Typography fontSize="inherit">
                    Gateway <code>{sp.gateway.slice(0, 10)}...</code>
                  </Typography>
                  <Typography fontSize="inherit">
                    Provider <code>{sp.address.slice(0, 10)}...</code>
                  </Typography>
                </Stack>
              }
              arrow
              placement="top"
            >
              <Typography>{description}</Typography>
            </Tooltip>
          </MenuItem>
        ))}
      </TextField>
    </Box>
  );
};
