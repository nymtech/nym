import * as React from 'react';
import { Link as RRDLink } from 'react-router-dom';
import { Box, Button, Card, Grid, Link as MuiLink, FormControl, Select, MenuItem } from '@mui/material';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { SelectChangeEvent } from '@mui/material/Select';
import { useMainContext } from '../../context/main';
import { gatewayToGridRow } from '../../components/Gateways';
import { GatewayResponse } from '../../typeDefs/explorer-api';
import { TableToolbar } from '../../components/TableToolbar';
import { CustomColumnHeading } from '../../components/CustomColumnHeading';
import { Title } from '../../components/Title';
import { cellStyles, UniversalDataGrid } from '../../components/Universal-DataGrid';
import { unymToNym } from '../../utils/currency';
import { Tooltip } from '../../components/Tooltip';
import { NYM_BIG_DIPPER } from '../../api/constants';
import { splice } from '../../utils';
import { VersionDisplaySelector, VersionSelectOptions } from '../../components/Gateways/VersionDisplaySelector';

export const PageGateways: FCWithChildren = () => {
  const { gateways } = useMainContext();
  const [filteredGateways, setFilteredGateways] = React.useState<GatewayResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('50');
  const [searchTerm, setSearchTerm] = React.useState<string>('');
  const [versionFilter, setVersionFilter] = React.useState<VersionSelectOptions>(VersionSelectOptions.latestVersion);

  const handleSearch = (str: string) => {
    setSearchTerm(str.toLowerCase());
  };

  React.useEffect(() => {
    const filteredByVersion = gateways?.data?.filter((g) => {
      if (versionFilter === 'Latest version') return versionToNumber(g.gateway.version) === 118;
      return versionToNumber(g.gateway.version) < 118;
    });

    if (searchTerm === '' && filteredByVersion) {
      setFilteredGateways(filteredByVersion);
    } else {
      const filtered = filteredByVersion?.filter((g) => {
        if (
          g.gateway.location.toLowerCase().includes(searchTerm) ||
          g.gateway.identity_key.toLocaleLowerCase().includes(searchTerm) ||
          g.owner.toLowerCase().includes(searchTerm)
        ) {
          return g;
        }
        return null;
      });

      if (filtered) {
        setFilteredGateways(filtered);
      }
    }
  }, [searchTerm, gateways?.data, versionFilter]);

  const versionToNumber = (version: string) => {
    const asNumber = Number(version.split('.').join(''));
    return asNumber;
  };

  const columns: GridColDef[] = [
    {
      field: 'identityKey',
      headerName: 'Identity Key',
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 380,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles }}
          component={RRDLink}
          to={`/network-components/gateway/${params.row.identityKey}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'bond',
      width: 150,
      type: 'number',
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" />,
      headerClassName: 'MuiDataGrid-header-override',
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles }}
          component={RRDLink}
          to={`/network-components/gateway/${params.row.identityKey}`}
          data-testid="pledge-amount"
        >
          {unymToNym(params.value, 6)}
        </MuiLink>
      ),
    },
    {
      field: 'performance',
      headerName: 'Routing Score',
      renderHeader: () => <CustomColumnHeading headingTitle="Routing Score" />,
      width: 150,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles }}
          component={RRDLink}
          to={`/network-components/gateway/${params.row.identityKey}`}
          data-testid="pledge-amount"
        >
          {`${params.value}%`}
        </MuiLink>
      ),
    },
    {
      field: 'host',
      renderHeader: () => <CustomColumnHeading headingTitle="IP:Port" />,
      width: 180,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles }}
          component={RRDLink}
          to={`/network-components/gateway/${params.row.identityKey}`}
          data-testid="host"
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      width: 180,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Button
          onClick={() => handleSearch(params.value as string)}
          sx={{ ...cellStyles, justifyContent: 'flex-start' }}
          data-testid="location-button"
        >
          <Tooltip text={params.value} id="gateway-location-text">
            <Box
              sx={{
                overflow: 'hidden',
                whiteSpace: 'nowrap',
                textOverflow: 'ellipsis',
              }}
            >
              {params.value}
            </Box>
          </Tooltip>
        </Button>
      ),
    },
    {
      field: 'owner',
      headerName: 'Owner',
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      width: 180,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles }}
          href={`${NYM_BIG_DIPPER}/account/${params.value}`}
          target="_blank"
          data-testid="owner"
        >
          {splice(7, 29, params.value)}
        </MuiLink>
      ),
    },
    {
      field: 'version',
      headerName: 'Version',
      renderHeader: () => <CustomColumnHeading headingTitle="Version" />,
      width: 150,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles }}
          href={`${NYM_BIG_DIPPER}/account/${params.value}`}
          target="_blank"
          data-testid="owner"
        >
          {params.value}
        </MuiLink>
      ),
    },
  ];

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  if (gateways?.data) {
    return (
      <>
        <Title text="Gateways" />
        <Grid container>
          <Grid item xs={12}>
            <Card
              sx={{
                padding: 2,
                height: '100%',
              }}
            >
              <TableToolbar
                onChangeSearch={handleSearch}
                onChangePageSize={handlePageSize}
                pageSize={pageSize}
                searchTerm={searchTerm}
                childrenBefore={
                  <VersionDisplaySelector
                    handleChange={(option) => setVersionFilter(option)}
                    selected={versionFilter}
                  />
                }
              />
              <UniversalDataGrid
                pagination
                rows={gatewayToGridRow(filteredGateways)}
                columns={columns}
                pageSize={pageSize}
              />
            </Card>
          </Grid>
        </Grid>
      </>
    );
  }
  return null;
};
