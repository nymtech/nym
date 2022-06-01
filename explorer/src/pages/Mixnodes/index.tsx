import * as React from 'react';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { Button, Card, Grid, Link as MuiLink } from '@mui/material';
import { Link as RRDLink, useParams } from 'react-router-dom';
import { SelectChangeEvent } from '@mui/material/Select';
import { SxProps } from '@mui/system';
import { Theme, useTheme } from '@mui/material/styles';
import { useHistory } from 'react-router';
import { CopyToClipboard } from '@nymproject/react';
import { useMainContext } from '../../context/main';
import { MixnodeRowType, mixnodeToGridRow } from '../../components/MixNodes';
import { TableToolbar } from '../../components/TableToolbar';
import { MixNodeResponse, MixnodeStatusWithAll, toMixnodeStatus } from '../../typeDefs/explorer-api';
import { BIG_DIPPER } from '../../api/constants';
import { CustomColumnHeading } from '../../components/CustomColumnHeading';
import { Title } from '../../components/Title';
import { cellStyles, UniversalDataGrid } from '../../components/Universal-DataGrid';
import { currencyToString } from '../../utils/currency';
import { splice } from '../../utils';
import { getMixNodeStatusColor } from '../../components/MixNodes/Status';
import { MixNodeStatusDropdown } from '../../components/MixNodes/StatusDropdown';

const getCellFontStyle = (theme: Theme, row: MixnodeRowType) => {
  const color = getMixNodeStatusColor(theme, row.status);
  return {
    fontWeight: 400,
    fontSize: 12,
    color,
  };
};

const getCellStyles = (theme: Theme, row: MixnodeRowType): SxProps => ({
  ...cellStyles,
  // TODO: should these be here, or change in `cellStyles`??
  ...getCellFontStyle(theme, row),
});

export const PageMixnodes: React.FC = () => {
  const { mixnodes, fetchMixnodes } = useMainContext();
  const [filteredMixnodes, setFilteredMixnodes] = React.useState<MixNodeResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [searchTerm, setSearchTerm] = React.useState<string>('');
  const theme = useTheme();
  const { status } = useParams<{ status: MixnodeStatusWithAll | undefined }>();
  const history = useHistory();

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
  }, [searchTerm, mixnodes?.data, mixnodes?.isLoading]);

  React.useEffect(() => {
    // when the status changes, get the mixnodes
    fetchMixnodes(toMixnodeStatus(status));
  }, [status]);

  const handleMixnodeStatusChanged = (newStatus?: MixnodeStatusWithAll) => {
    history.push(
      newStatus && newStatus !== MixnodeStatusWithAll.all
        ? `/network-components/mixnodes/${newStatus}`
        : '/network-components/mixnodes',
    );
  };

  const columns: GridColDef[] = [
    {
      field: 'owner',
      headerName: 'Owner',
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      width: 200,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          href={`${BIG_DIPPER}/account/${params.value}`}
          target="_blank"
          sx={getCellStyles(theme, params.row)}
          data-testid="big-dipper-link"
        >
          {splice(7, 29, params.value)}
        </MuiLink>
      ),
    },
    {
      field: 'identity_key',
      headerName: 'Identity Key',
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 180,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <>
          <CopyToClipboard
            sx={{ ...getCellFontStyle(theme, params.row), mr: 1 }}
            value={params.value}
            tooltip={`Copy identity key ${params.value} to clipboard`}
          />
          <MuiLink
            sx={getCellStyles(theme, params.row)}
            component={RRDLink}
            to={`/network-components/mixnode/${params.value}`}
            data-testid="identity-link"
          >
            {splice(7, 29, params.value)}
          </MuiLink>
        </>
      ),
    },
    {
      field: 'bond',
      headerName: 'Bond',
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" />,
      type: 'number',
      headerClassName: 'MuiDataGrid-header-override',
      width: 200,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.identity_key}`}
        >
          {currencyToString(params.value)}
        </MuiLink>
      ),
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
          sx={{
            ...getCellStyles(theme, params.row),
            justifyContent: 'flex-start',
          }}
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
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.identity_key}`}
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
      width: 130,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.identity_key}`}
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
          sx={{ ...getCellStyles(theme, params.row), textAlign: 'left' }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.identity_key}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'profit_percentage',
      headerName: 'Profit Margin',
      renderHeader: () => <CustomColumnHeading headingTitle="Profit Margin" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 140,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...getCellStyles(theme, params.row), textAlign: 'left' }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.identity_key}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'avg_uptime',
      headerName: 'Average Uptime',
      renderHeader: () => <CustomColumnHeading headingTitle="Average Uptime" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 160,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...getCellStyles(theme, params.row), textAlign: 'left' }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.identity_key}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'stake_saturation',
      headerName: 'Stake Saturation',
      renderHeader: () => <CustomColumnHeading headingTitle="Stake Saturation" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 175,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{
            textAlign: 'left',
            color: params.value > 100 ? theme.palette.warning.main : 'inherit',
            ...getCellStyles(theme, params.row),
          }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.identity_key}`}
        >
          {`${params.value.toFixed(2)} %`}
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
            }}
          >
            <TableToolbar
              childrenBefore={
                <MixNodeStatusDropdown sx={{ mr: 2 }} status={status} onSelectionChanged={handleMixnodeStatusChanged} />
              }
              onChangeSearch={handleSearch}
              onChangePageSize={handlePageSize}
              pageSize={pageSize}
              searchTerm={searchTerm}
            />
            <UniversalDataGrid
              pagination
              loading={Boolean(mixnodes?.isLoading)}
              rows={mixnodeToGridRow(filteredMixnodes)}
              columns={columns}
              pageSize={pageSize}
            />
          </Card>
        </Grid>
      </Grid>
    </>
  );
};
