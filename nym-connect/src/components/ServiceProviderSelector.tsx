import React from 'react';
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ArrowDropDownCircleIcon from '@mui/icons-material/ArrowDropDownCircle';
import { Box, CircularProgress, Stack, Tooltip, Typography } from '@mui/material';
import { ServiceProvider, Services } from '../types/directory';

export const ServiceProviderSelector: React.FC<{
  onChange?: (serviceProvider: ServiceProvider) => void;
  services?: Services;
}> = ({ services, onChange }) => {
  const [serviceProvider, setServiceProvider] = React.useState<ServiceProvider | undefined>();
  const textEl = React.useRef<null | HTMLElement>(null);
  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null);
  const open = Boolean(anchorEl);
  const handleClick = () => {
    setAnchorEl(textEl.current);
  };
  const handleClose = (newServiceProvider?: ServiceProvider) => {
    if (newServiceProvider) {
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
        MenuListProps={{
          'aria-labelledby': 'service-provider-button',
        }}
      >
        {services.map((service) => (
          <>
            <MenuItem disabled dense sx={{ fontSize: 'small', fontWeight: 'bold', mb: -1 }}>
              {service.description}
            </MenuItem>
            {service.items.map((sp) => (
              <MenuItem dense sx={{ fontSize: 'small', ml: 2, height: 'auto' }} onClick={() => handleClose(sp)}>
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
                  <Typography fontSize="inherit" noWrap>
                    {sp.description}
                  </Typography>
                </Tooltip>
              </MenuItem>
            ))}
          </>
        ))}
      </Menu>
    </>
  );
};
