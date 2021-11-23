import * as React from 'react';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { printableCoin } from '@nymproject/nym-validator-client';
import { Button, Grid, Link as MuiLink, Card } from '@mui/material';
import { Link as RRDLink } from 'react-router-dom';
import { SelectChangeEvent } from '@mui/material/Select';
import { useMainContext } from 'src/context/main';
import { mixnodeToGridRow } from 'src/utils';
import { TableToolbar } from 'src/components/TableToolbar';
import { MixNodeResponse } from 'src/typeDefs/explorer-api';
import { BIG_DIPPER } from 'src/api/constants';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';
import { Title } from 'src/components/Title';
import {
  NewniversalDataGrid,
  cellStyles,
} from 'src/components/Newniversal-DataGrid';

export const PageMixnodes: React.FC = () => {
  const { mixnodes } = useMainContext();
  const [filteredMixnodes, setFilteredMixnodes] =
    React.useState<MixNodeResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [searchTerm, setSearchTerm] = React.useState<string>('');

  const handleSearch = (str: string) => {
    setSearchTerm(str.toLowerCase());
  };

  React.useEffect(() => {
    if (searchTerm === '' && mixnodes?.data) {
      setFilteredMixnodes(mixnodes?.data);
    } else {
      const filtered = mixnodes?.data?.filter((m) => {
        if (
          m.location?.country_name.toLowerCase().includes(searchTerm) ||
          m.mix_node.identity_key.toLocaleLowerCase().includes(searchTerm) ||
          m.owner.toLowerCase().includes(searchTerm)
        ) {
          return m;
        }
        return null;
      });
      if (filtered) {
        setFilteredMixnodes(filtered);
      }
    }
  }, [searchTerm, mixnodes?.data]);

  const columns: GridColDef[] = [
    {
      field: 'owner',
      headerName: 'Owner',
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      width: 380,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          href={`${BIG_DIPPER}/account/${params.value}`}
          target="_blank"
          sx={cellStyles}
          data-testid="big-dipper-link"
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'identity_key',
      headerName: 'Identity Key',
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      width: 380,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={cellStyles}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.value}`}
          data-testid="identity-link"
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'bond',
      headerName: 'Bond',
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" />,
      type: 'number',
      headerClassName: 'MuiDataGrid-header-override',
      width: 150,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => {
        const bondAsPunk = printableCoin({
          amount: params.value as string,
          denom: 'upunk',
        });
        return (
          <MuiLink
            sx={cellStyles}
            component={RRDLink}
            to={`/network-components/mixnodes/${params.row.identity_key}`}
          >
            {bondAsPunk}
          </MuiLink>
        );
      },
    },
    {
      field: 'location',
      headerName: 'Location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      width: 150,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Button
          onClick={() => handleSearch(params.value as string)}
          sx={{ ...cellStyles, justifyContent: 'flex-start' }}
        >
          {params.value}
        </Button>
      ),
    },
    {
      field: 'self_percentage',
      headerName: 'Self %',
      width: 110,
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle="Self %" />,
      type: 'number',
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={cellStyles}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {params.value}%
        </MuiLink>
      ),
    },
    {
      field: 'host',
      headerName: 'Host',
      renderHeader: () => <CustomColumnHeading headingTitle="Host" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 110,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={cellStyles}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'layer',
      headerName: 'Layer',
      renderHeader: () => <CustomColumnHeading headingTitle="Layer" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 110,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles, textAlign: 'left' }}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
  ];

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  return (
    <>
      <Title text="Mixnodes" />
      <Grid container>
        <Grid item xs={12}>
          <Card
            sx={{
              padding: 2,
              height: '100%',
              // border: '1px solid blue',
            }}
          >
            <TableToolbar
              onChangeSearch={handleSearch}
              onChangePageSize={handlePageSize}
              pageSize={pageSize}
              searchTerm={searchTerm}
            />
            <NewniversalDataGrid
              pagination
              rows={mixnodeToGridRow(filteredMixnodes)}
              columns={columns}
              hideFooter={false}
              pageSize={pageSize}
            />
          </Card>
        </Grid>
      </Grid>
    </>
  );
};
