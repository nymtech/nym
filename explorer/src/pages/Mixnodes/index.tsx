import * as React from 'react';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { Button, Card, Grid, Link as MuiLink } from '@mui/material';
import { Link as RRDLink, useParams, useNavigate } from 'react-router-dom';
import { SelectChangeEvent } from '@mui/material/Select';
import { SxProps } from '@mui/system';
import { Theme, useTheme } from '@mui/material/styles';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
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

const getCellFontStyle = (theme: Theme, row: MixnodeRowType, textColor?: string) => {
  const color = textColor || getMixNodeStatusColor(theme, row.status);
  return {
    fontWeight: 400,
    fontSize: 12,
    color,
  };
};

const getCellStyles = (theme: Theme, row: MixnodeRowType, textColor?: string): SxProps => ({
  ...cellStyles,
  // TODO: should these be here, or change in `cellStyles`??
  ...getCellFontStyle(theme, row, textColor),
});

export const PageMixnodes: React.FC = () => {
  const { mixnodes, fetchMixnodes } = useMainContext();
  const [filteredMixnodes, setFilteredMixnodes] = React.useState<MixNodeResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [searchTerm, setSearchTerm] = React.useState<string>('');
  const theme = useTheme();
  const { status } = useParams<{ status: MixnodeStatusWithAll | undefined }>();
  const navigate = useNavigate();

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
    navigate(
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
      field: 'bond',
      headerName: 'Stake',
      renderHeader: () => <CustomColumnHeading headingTitle="Stake" />,
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
      field: 'stake_saturation',
      headerName: 'Stake Saturation',
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Stake Saturation"
          tooltipInfo="Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is: 1 million NYM, computed as S/K where S is  total amount of tokens available to stakeholders and K is the number of nodes in the reward set."
        />
      ),
      headerClassName: 'MuiDataGrid-header-override',
      width: 190,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{
            textAlign: 'left',
            ...getCellStyles(theme, params.row, params.value > 100 ? 'theme.palette.warning.main' : undefined),
          }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.identity_key}`}
        >
          {`${params.value.toFixed(2)} %`}
        </MuiLink>
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
      field: 'profit_percentage',
      headerName: 'Profit Margin',
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Profit Margin"
          tooltipInfo="Percentage of the delegates rewards that the operator takes as fee before rewards are distributed to the delegates."
        />
      ),
      headerClassName: 'MuiDataGrid-header-override',
      width: 165,
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
      headerName: 'Routing Score',
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Routing Score"
          tooltipInfo="Node’s routing score is relative to that of the network. Each time a node is tested, the test packets have to go through the full path of the network (a gateway + 3 nodes). If a node in the path drop packets it will affect the score of other nodes in the test."
        />
      ),
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
