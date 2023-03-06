import React, { useEffect, useState } from 'react';
import { Card, FormControl, Grid, MenuItem, Select, SelectChangeEvent } from '@mui/material';
import { Api } from '../../api';
import { TableToolbar } from '../../components/TableToolbar';
import { Title } from '../../components/Title';
import { UniversalDataGrid } from '../../components/Universal-DataGrid';
import { DirectoryService } from '../../typeDefs/explorer-api';

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
        <MenuItem sx={{ opacity: 1 }}>Keybase</MenuItem>
        <MenuItem>Telegram</MenuItem>
        <MenuItem>Electrum</MenuItem>
      </Select>
    </FormControl>
  );
};

export const ServiceProviders = () => {
  const [serviceProviders, setServiceProviders] = useState<DirectoryService>();
  const [pageSize, setPageSize] = React.useState('10');

  const getServiceproviders = async () => {
    const [data] = await Api.fetchServiceProviders();
    setServiceProviders(data);
  };

  useEffect(() => {
    getServiceproviders();
  }, []);

  const handleOnPageSizeChange = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  if (!serviceProviders) return null;

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
            <TableToolbar
              onChangePageSize={handleOnPageSizeChange}
              pageSize={pageSize}
              childrenBefore={<SupportedApps />}
            />
            <UniversalDataGrid pagination rows={serviceProviders.items} columns={columns} pageSize={pageSize} />
          </Card>
        </Grid>
      </Grid>
    </>
  );
};
