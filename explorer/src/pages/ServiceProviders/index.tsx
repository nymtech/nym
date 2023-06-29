import React from 'react';
import { Button, Card, FormControl, Grid, ListItem, Menu, SelectChangeEvent, Typography } from '@mui/material';
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
  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null);
  const open = Boolean(anchorEl);
  const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    setAnchorEl(event.currentTarget);
  };
  const handleClose = () => {
    setAnchorEl(null);
  };
  const anchorRef = React.useRef<HTMLButtonElement>(null);

  return (
    <FormControl size="small">
      <Button
        ref={anchorRef}
        onClick={handleClick}
        size="large"
        variant="outlined"
        color="inherit"
        sx={{ mr: 2, textTransform: 'capitalize' }}
      >
        Supported Apps
      </Button>
      <Menu id="basic-menu" anchorEl={anchorEl} open={open} onClose={handleClose}>
        <ListItem>Keybase</ListItem>
        <ListItem>Telegram</ListItem>
        <ListItem>Electrum</ListItem>
        <ListItem>Blockstream Green</ListItem>
      </Menu>
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
            {serviceProviders?.data ? (
              <>
                <TableToolbar
                  onChangePageSize={handleOnPageSizeChange}
                  pageSize={pageSize}
                  childrenBefore={<SupportedApps />}
                />
                <UniversalDataGrid
                  pagination
                  rows={serviceProviders?.data?.items}
                  columns={columns}
                  pageSize={pageSize}
                />
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
