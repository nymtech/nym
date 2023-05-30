import React, { useState } from 'react';
import { Card, FormControl, Grid, MenuItem, Select, SelectChangeEvent, Typography } from '@mui/material';
import { TableToolbar } from '../../components/TableToolbar';
import { Title } from '../../components/Title';
import { UniversalDataGrid } from '../../components/Universal-DataGrid';
import { useMainContext } from '../../context/main';

const columns = [
  {
    headerName: 'Client ID',
    field: 'address',
    disableColumnMenu: true,
    flex: 1,
  },
];

const SupportedApps = () => {
  const [selected, setSelected] = useState<string>('');
  const handleChange = (e: SelectChangeEvent) => setSelected(e.target.value);
  return (
    <FormControl size="small">
      <Select value={selected} onChange={handleChange} displayEmpty sx={{ mr: 2 }}>
        <MenuItem value="">Supported Apps</MenuItem>
        <MenuItem>Keybase</MenuItem>
        <MenuItem>Telegram</MenuItem>
        <MenuItem>Electrum</MenuItem>
        <MenuItem>Blockstream Green</MenuItem>
      </Select>
    </FormControl>
  );
};

export const ServiceProviders = () => {
  const { serviceProviders } = useMainContext();
  const [pageSize, setPageSize] = React.useState('10');

  const handleOnPageSizeChange = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  return (
    <>
      <Title text="Service Providers" />
      <Grid container>
        <Grid item xs={12}>
          <Card
            sx={{
              padding: 2,
            }}
          >
            {serviceProviders ? (
              <>
                <TableToolbar
                  onChangePageSize={handleOnPageSizeChange}
                  pageSize={pageSize}
                  childrenBefore={<SupportedApps />}
                />
                <UniversalDataGrid pagination rows={serviceProviders} columns={columns} pageSize={pageSize} />
              </>
            ) : (
              <Typography>No service providers to display</Typography>
            )}
          </Card>
        </Grid>
      </Grid>
    </>
  );
};
