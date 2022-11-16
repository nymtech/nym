import React, { useEffect, useMemo } from 'react';
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import KeyboardArrowDownRoundedIcon from '@mui/icons-material/KeyboardArrowDownRounded';
import KeyboardArrowUpRoundedIcon from '@mui/icons-material/KeyboardArrowUpRounded';
import { Box, CircularProgress, Stack, Tooltip, Typography, ListItemIcon } from '@mui/material';
import Check from '@mui/icons-material/Check';
import { ServiceProvider, Service, Services } from '../types/directory';

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
  const [service, setService] = React.useState<Service>();
  const [serviceProvider, setServiceProvider] = React.useState<ServiceProvider | undefined>(currentSp);
  const textEl = React.useRef<null | HTMLElement>(null);
  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null);
  const open = Boolean(anchorEl);

  useEffect(() => {
    if (!serviceProvider && currentSp) {
      setServiceProvider(currentSp);
    }
  }, [currentSp]);

  useEffect(() => {
    if (services && serviceProvider) {
      // retrieve the service corresponding to this service provider
      setService(
        services.find((s) =>
          s.items.some(
            ({ id, address, gateway }) =>
              id === serviceProvider.id && address === serviceProvider.address && gateway === serviceProvider.gateway,
          ),
        ),
      );
    }
  }, [serviceProvider, services]);

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
          {open ? <KeyboardArrowUpRoundedIcon /> : <KeyboardArrowDownRoundedIcon />}
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
      <Box
        display="flex"
        alignItems="center"
        justifyContent="space-between"
        sx={{ mt: 3, borderBottom: (theme) => `1px solid ${theme.palette.info.main}` }}
      >
        <Typography ref={textEl} fontSize={14} fontWeight={700} color={(theme) => theme.palette.info.main}>
          {!service ? 'Select a service' : service.description}
        </Typography>
        <IconButton
          id="service-provider-button"
          aria-controls={open ? 'basic-menu' : undefined}
          aria-haspopup="true"
          aria-expanded={open ? 'true' : undefined}
          onClick={handleClick}
          color="info"
          size="small"
          sx={{ padding: 0 }}
        >
          {open ? <KeyboardArrowUpRoundedIcon /> : <KeyboardArrowDownRoundedIcon />}
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
        PaperProps={{
          sx: {
            border: '1px solid rgba(96, 214, 239, 0.4)',
          },
        }}
        MenuListProps={{
          'aria-labelledby': 'service-provider-button',
          sx: {
            minWidth: 160,
          },
        }}
      >
        {servicesWithRandomSp.map(({ id, description, sp }) => (
          <MenuItem
            dense
            autoFocus={id === service?.id}
            key={id}
            sx={{
              fontSize: 'small',
              fontWeight: 'bold',
              minWidth: '208px',
              '&.Mui-focusVisible': { bgcolor: 'transparent' },
            }}
            onClick={() => handleClose(sp)}
          >
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
            {id === service?.id && (
              <ListItemIcon
                sx={{
                  position: 'absolute',
                  right: '0',
                }}
              >
                <Check sx={{ padding: 0 }} />
              </ListItemIcon>
            )}
          </MenuItem>
        ))}
      </Menu>
    </>
  );
};
