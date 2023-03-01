import * as React from 'react';
import { Link as RRDLink } from 'react-router-dom';
import { Box, Card, Grid, Link as MuiLink } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { SelectChangeEvent } from '@mui/material/Select';
import { diff, rcompare } from 'semver';
import { useMainContext } from '../../context/main';
import { gatewayToGridRow } from '../../components/Gateways';
import { GatewayResponse } from '../../typeDefs/explorer-api';
import { TableToolbar } from '../../components/TableToolbar';
import { CustomColumnHeading } from '../../components/CustomColumnHeading';
import { Title } from '../../components/Title';
import { cellStyles, UniversalDataGrid } from '../../components/Universal-DataGrid';
import { unymToNym } from '../../utils/currency';
import { Tooltip } from '../../components/Tooltip';
import { Tooltip as InfoTooltip } from '@nymproject/react/tooltip/Tooltip';
import { NYM_BIG_DIPPER } from '../../api/constants';
import { splice } from '../../utils';
import { VersionDisplaySelector, VersionSelectOptions } from '../../components/Gateways/VersionDisplaySelector';

export const PageGateways: FCWithChildren = () => {
  const { gateways } = useMainContext();
  const [filteredGateways, setFilteredGateways] = React.useState<GatewayResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('50');
  const [searchTerm, setSearchTerm] = React.useState<string>('');
  const [versionFilter, setVersionFilter] = React.useState<VersionSelectOptions>(VersionSelectOptions.latestVersion);

  const theme = useTheme();

  const handleSearch = (str: string) => {
    setSearchTerm(str.toLowerCase());
  };

  const highestVersion = React.useMemo(() => {
    if (gateways?.data) {
      const versions = gateways.data.reduce((a: string[], b) => [...a, b.gateway.version], []);
      const [lastestVersion] = versions.sort(rcompare);
      return lastestVersion;
    }
    // fallback value
    return '2.0.0';
  }, [gateways]);

  const filterByLatestVersions = React.useMemo(() => {
    const filtered = gateways?.data?.filter((gw) => {
      const versionDiff = diff(highestVersion, gw.gateway.version);
      return versionDiff === 'patch' || versionDiff === null;
    });
    if (filtered) return filtered;
    return [];
  }, [gateways]);

  const filterByOlderVersions = React.useMemo(() => {
    const filtered = gateways?.data?.filter((gw) => {
      const versionDiff = diff(highestVersion, gw.gateway.version);
      return versionDiff === 'major' || versionDiff === 'minor';
    });
    if (filtered) return filtered;
    return [];
  }, [gateways]);

  const filteredByVersion = React.useMemo(
    () => (versionFilter === VersionSelectOptions.latestVersion ? filterByLatestVersions : filterByOlderVersions),
    [versionFilter, gateways],
  );

  React.useEffect(() => {
    if (searchTerm === '') {
      setFilteredGateways(filteredByVersion);
    } else {
      const filtered = filteredByVersion.filter((g) => {
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

  const columns: GridColDef[] = [
    {
      field: 'identity_key',
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 380,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <>
          <CopyToClipboard
            sx={{ mr: 1, fontSize: 12 }}
            value={params.value}
            tooltip={`Copy identity key ${params.value} to clipboard`}
          />
          <MuiLink
            sx={{ ...cellStyles }}
            component={RRDLink}
            to={`/network-components/gateway/${params.row.identity_key}`}
          >
            {params.value}
          </MuiLink>
        </>
      ),
    },
    {
      field: 'node_performance',
      renderHeader: () => (
        <>
          <InfoTooltip
            id="gateways-list-routing-score"
            title="Gateway's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test"
            placement="top-start"
            textColor={theme.palette.nym.networkExplorer.tooltip.color}
            bgColor={theme.palette.nym.networkExplorer.tooltip.background}
            maxWidth={230}
            arrow
          />
          <CustomColumnHeading headingTitle="Routing Score" />,
        </>
      ),
      width: 150,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles }}
          component={RRDLink}
          to={`/network-components/gateway/${params.row.identity_key}`}
          data-testid="pledge-amount"
        >
          {`${params.value}%`}
        </MuiLink>
      ),
    },
    {
      field: 'version',
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
    {
      field: 'location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      width: 180,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Box
          onClick={() => handleSearch(params.value as string)}
          sx={{ ...cellStyles, justifyContent: 'flex-start', cursor: 'pointer' }}
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
        </Box>
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
          to={`/network-components/gateway/${params.row.identity_key}`}
          data-testid="host"
        >
          {params.value}
        </MuiLink>
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
          to={`/network-components/gateway/${params.row.identity_key}`}
          data-testid="pledge-amount"
        >
          {unymToNym(params.value, 6)}
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
