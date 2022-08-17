import React, { useEffect, useMemo } from 'react';
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ArrowDropDownCircleIcon from '@mui/icons-material/ArrowDropDownCircle';
import { Box, CircularProgress, Stack, Tooltip, Typography } from '@mui/material';
import { ServiceProvider, Services } from '../types/directory';

type ServiceWithRandomSp = {
  id: string;
  description: string;
  sp: ServiceProvider;
};

export const ServiceProviderSelector: React.FC<{
  onChange?: (serviceProvider: ServiceProvider) => void;
  services?: Services;
  currentSp?: ServiceProvider;
}> = ({ services, currentSp, onChange }) => {
  const [serviceProvider, setServiceProvider] = React.useState<ServiceProvider | undefined>(currentSp);
  const textEl = React.useRef<null | HTMLElement>(null);
  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null);
  const open = Boolean(anchorEl);

  useEffect(() => {
    if (!serviceProvider && currentSp) {
      setServiceProvider(currentSp);
    }
  }, [currentSp]);

  const handleClick = () => {
    setAnchorEl(textEl.current);
  };
  const handleClose = (newServiceProvider?: ServiceProvider) => {
    if (newServiceProvider && newServiceProvider !== currentSp) {
      setServiceProvider(newServiceProvider);
      onChange?.(newServiceProvider);
    }
    setAnchorEl(null);
  };

  if (!services) {
    return (
      <Box display="flex" alignItems="center" justifyContent="space-between" sx={{ mt: 3 }}>
        <Typography fontSize={14} fontWeight={700} color={(theme) => theme.palette.common.white}>
          <CircularProgress size={14} sx={{ mr: 1 }} color="inherit" />
          Loading services...
        </Typography>
        <IconButton id="service-provider-button" disabled>
          <ArrowDropDownCircleIcon />
        </IconButton>
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
    [services],
  );

  return (
    <>
      <Box display="flex" alignItems="center" justifyContent="space-between" sx={{ mt: 3 }}>
        <Typography
          ref={textEl}
          fontSize={14}
          fontWeight={700}
          color={(theme) => (serviceProvider ? undefined : theme.palette.primary.main)}
        >
          {serviceProvider ? serviceProvider.description : 'Select a service'}
        </Typography>
        <IconButton
          id="service-provider-button"
          aria-controls={open ? 'basic-menu' : undefined}
          aria-haspopup="true"
          aria-expanded={open ? 'true' : undefined}
          onClick={handleClick}
        >
          <ArrowDropDownCircleIcon />
        </IconButton>
      </Box>
      <Menu
        id="service-provider-menu"
        anchorEl={anchorEl}
        open={open}
        onClose={() => handleClose()}
        anchorOrigin={{
          vertical: 'bottom',
          horizontal: 'right',
        }}
        transformOrigin={{
          vertical: 'top',
          horizontal: 'left',
        }}
        MenuListProps={{
          'aria-labelledby': 'service-provider-button',
          sx: {
            minWidth: 160,
          },
        }}
      >
        {servicesWithRandomSp.map(({ id, description, sp }) => (
          <MenuItem dense key={id} sx={{ fontSize: 'small', fontWeight: 'bold' }} onClick={() => handleClose(sp)}>
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
      </Menu>
    </>
  );
};
