import React from 'react';
import { Autocomplete, Box, Dialog, DialogProps, TextField, Typography } from '@mui/material';
import { Service, ServiceProvider, Services } from '../types/directory';

export const ServiceProviderPopup: FCWithChildren<
  DialogProps & { services: Services; onServiceProviderChanged: (sp?: ServiceProvider, s?: Service) => void }
> = ({ services, onServiceProviderChanged, ...dialogProps }) => {
  const options = services.flatMap((s) =>
    s.items.map((sp) => ({ id: `${s.id}-${sp.id}`, title: sp.description, service: s, sp })),
  );
  return (
    <Dialog {...dialogProps} fullWidth PaperProps={{ sx: { p: 0, m: 0, width: '100%' } }}>
      <Autocomplete
        fullWidth
        openOnFocus
        sx={{ p: 1 }}
        // filterOptions={(filterOptions, { inputValue }) =>
        //   filterOptions.filter((o) => o.title.toLowerCase().includes(inputValue.toLowerCase()))
        // }
        options={options}
        groupBy={(option) => option.service.description}
        getOptionLabel={(option) => option.title}
        onChange={(event, value) => onServiceProviderChanged(value?.sp, value?.service)}
        isOptionEqualToValue={(option, value) => option.id.toLowerCase() === value?.id.toLowerCase()}
        renderOption={(props, option) => (
          <Box key={option.id} component="li" sx={{ '& > img': { mr: 2, flexShrink: 0 } }} {...props} fontSize="small">
            <Typography component="p" sx={{ opacity: 0.5, mr: 2, fontSize: 'inherit' }}>
              {option.id}
            </Typography>
            <p>{option.title}</p>
          </Box>
        )}
        renderInput={(params) => <TextField autoFocus {...params} label="Select a service provider" />}
      />
    </Dialog>
  );
};
