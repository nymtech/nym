import React, { useEffect } from 'react';
import {
  Box,
  CircularProgress,
  Divider,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Typography,
} from '@mui/material';
import { Service, ServiceProvider, Services } from '../types/directory';
import { useTauriEvents } from '../utils';
import { ServiceProviderPopup } from './ServiceProviderPopup';

export const ServiceSelector: React.FC<{
  onChange?: (serviceProvider?: ServiceProvider) => void;
  services?: Services;
  currentSp?: ServiceProvider;
}> = ({ services, currentSp, onChange }) => {
  const [service, setService] = React.useState<Service | undefined>();
  const [serviceProvider, setServiceProvider] = React.useState<ServiceProvider | undefined>(currentSp);
  const [isPopupVisible, setPopupVisible] = React.useState(false);

  const getService = () => {
    if (!services || !currentSp) {
      return undefined;
    }
    return services.find((s) =>
      s.items.some(
        ({ id, address, gateway }) =>
          id === currentSp.id && address === currentSp.address && gateway === currentSp.gateway,
      ),
    );
  };

  useEffect(() => {
    if (!service && currentSp) {
      setServiceProvider(currentSp);
      setService(getService());
    }
  }, [currentSp, services]);

  /**
   * Gets a random service provider from the currently selected service.
   *
   * If there is no service selected, or it does not have items, `undefined` is returned.
   */
  const getRandomServiceProviderForService = (serviceToUse?: Service): ServiceProvider | undefined => {
    if (!serviceToUse?.items.length) {
      return undefined;
    }
    return serviceToUse.items[Math.floor(Math.random() * serviceToUse.items.length)];
  };

  const handleServiceSelected = React.useCallback(
    (newService?: Service) => {
      console.log(newService?.id, service?.id);
      // if the user has chosen a new service, then pick a random service provider
      if (newService?.id !== service?.id) {
        const newServiceProvider = getRandomServiceProviderForService(newService);
        setServiceProvider(newServiceProvider);
        onChange?.(newServiceProvider);
        setService(newService);
      }
    },
    [service],
  );

  // clears the display and fire on change (to trigger upstream storage clearance)
  const clearServiceProviderAndFireOnChange = () => {
    setService(undefined);
    setServiceProvider(undefined);
    onChange?.(undefined);
  };

  // when the user clears local storage, reset the selector
  useTauriEvents('help://clear-storage', () => {
    clearServiceProviderAndFireOnChange();
  });

  const handleAdvancedSpChange = (newServiceProvider?: ServiceProvider, newService?: Service) => {
    setPopupVisible(false);
    setService(newService);
    setServiceProvider(newServiceProvider);
    onChange?.(newServiceProvider);
  };

  const handleNewService = (newServiceId?: string) => {
    const newService = (services || []).find((s) => s.id === newServiceId);
    setService(newService);
  };

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

  return (
    <Box display="flex" alignItems="center" justifyContent="space-between" sx={{ my: 3 }}>
      <FormControl fullWidth>
        <InputLabel
          id="service-label"
          sx={{
            color: 'grey.500',
            '&.MuiInputLabel-shrink': {
              marginTop: '16px',
              marginLeft: '-2px',
            },
            '&.Mui-focused': {
              color: 'grey.500',
            },
          }}
        >
          Select a service
        </InputLabel>
        <Select
          labelId="service-label"
          id="service-id"
          variant="filled"
          value={service?.id || ''}
          onChange={(event) => handleNewService(event.target.value)}
          fullWidth
          MenuProps={{
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
          }}
        >
          {serviceProvider && (
            <Box px={2} pb={1} sx={{ opacity: 0.5 }}>
              <Stack direction="column" fontSize="small">
                <Typography fontSize="inherit">{serviceProvider.description}</Typography>
                <Typography fontSize="inherit">
                  <code>{serviceProvider.id}</code>
                </Typography>
              </Stack>
            </Box>
          )}
          {serviceProvider && <Divider />}
          {services.map((item) => (
            <MenuItem key={item.id} value={item.id} onClick={() => handleServiceSelected(item)}>
              <Typography>{item.description}</Typography>
            </MenuItem>
          ))}
          <Divider />
          <Typography ml={2} variant="overline" display="block" sx={{ opacity: 0.5 }}>
            Advanced
          </Typography>
          <MenuItem onClick={() => setPopupVisible(true)}>Choose service provider</MenuItem>
          <MenuItem onClick={clearServiceProviderAndFireOnChange}>Clear settings</MenuItem>
        </Select>
      </FormControl>
      <ServiceProviderPopup
        open={isPopupVisible}
        services={services}
        onBackdropClick={() => setPopupVisible(false)}
        onServiceProviderChanged={handleAdvancedSpChange}
      />
    </Box>
  );
};
