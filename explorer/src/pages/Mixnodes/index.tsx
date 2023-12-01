import * as React from 'react';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { Button, Card, Grid, Link as MuiLink, Box } from '@mui/material';
import { Link as RRDLink, useParams, useNavigate } from 'react-router-dom';
import { SelectChangeEvent } from '@mui/material/Select';
import { SxProps } from '@mui/system';
import { Theme, useTheme } from '@mui/material/styles';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { useWallet } from '@cosmos-kit/react';
import { useMainContext } from '../../context/main';
import { MixnodeRowType, mixnodeToGridRow } from '../../components/MixNodes';
import { TableToolbar } from '../../components/TableToolbar';
import { MixNodeResponse, MixnodeStatusWithAll, toMixnodeStatus } from '../../typeDefs/explorer-api';
import { NYM_BIG_DIPPER } from '../../api/constants';
import { CustomColumnHeading } from '../../components/CustomColumnHeading';
import { Title } from '../../components/Title';
import { cellStyles, UniversalDataGrid } from '../../components/Universal-DataGrid';
import { currencyToString } from '../../utils/currency';
import { splice } from '../../utils';
import { getMixNodeStatusColor } from '../../components/MixNodes/Status';
import { MixNodeStatusDropdown } from '../../components/MixNodes/StatusDropdown';
import { Tooltip } from '../../components/Tooltip';
import { DelegateIconButton } from '../../components/Delegations/components/DelegateIconButton';
import { DelegationModal, DelegationModalProps } from '../../components/Delegations/components/DelegationModal';
import { DelegateModal } from '../../components/Delegations/components/DelegateModal';

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

export const PageMixnodes: FCWithChildren = () => {
  const { mixnodes, fetchMixnodes } = useMainContext();
  const [filteredMixnodes, setFilteredMixnodes] = React.useState<MixNodeResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [searchTerm, setSearchTerm] = React.useState<string>('');
  const [itemSelectedForDelegation, setItemSelectedForDelegation] = React.useState<{
    mixId: number;
    identityKey: string;
  }>();
  const [confirmationModalProps, setConfirmationModalProps] = React.useState<DelegationModalProps | undefined>();
  const { status } = useParams<{ status: MixnodeStatusWithAll | undefined }>();

  const theme = useTheme();
  const navigate = useNavigate();
  const wallet = useWallet();

  const handleNewDelegation = (delegationModalProps: DelegationModalProps) => {
    setItemSelectedForDelegation(undefined);
    setConfirmationModalProps(delegationModalProps);
  };

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
      field: 'delegate',
      headerName: 'Delegate',
      disableColumnMenu: true,
      disableReorder: true,
      align: 'center',
      renderHeader: () => <CustomColumnHeading headingTitle="Delegate" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 80,
      headerAlign: 'center',
      renderCell: (params: GridRenderCellParams) => (
        <DelegateIconButton
          onDelegate={() => {
            if (wallet.status !== 'Connected') {
              setConfirmationModalProps({
                status: 'info',
                message: 'Please connect your wallet to delegate',
              });
            } else {
              setItemSelectedForDelegation({ identityKey: params.row.identity_key, mixId: params.row.mix_id });
            }
          }}
        />
      ),
    },
    {
      field: 'mix_id',
      headerName: 'Mix ID',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Mix ID" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 70,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnode/${params.value}`}
          data-testid="mix-id"
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'identity_key',
      headerName: 'Identity Key',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 190,
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
            to={`/network-components/mixnode/${params.row.mix_id}`}
            data-testid="identity-link"
          >
            {splice(7, 29, params.value)}
          </MuiLink>
        </>
      ),
    },
    {
      field: 'bond',
      headerName: 'Stake',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Stake" />,
      type: 'number',
      headerClassName: 'MuiDataGrid-header-override',
      width: 170,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.mix_id}`}
        >
          {currencyToString({ amount: params.value, dp: 2 })}
        </MuiLink>
      ),
    },
    {
      field: 'stake_saturation',
      headerName: 'Stake Saturation',
      disableColumnMenu: true,
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Stake Saturation"
          tooltipInfo="Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is 940k NYMs, computed as S/K where S is target amount of tokens staked in the network and K is the number of nodes in the reward set."
        />
      ),
      headerClassName: 'MuiDataGrid-header-override',
      width: 185,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{
            textAlign: 'left',
            ...getCellStyles(theme, params.row, params.value > 100 ? 'theme.palette.warning.main' : undefined),
          }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.mix_id}`}
        >
          {`${params.value} %`}
        </MuiLink>
      ),
    },
    {
      field: 'pledge_amount',
      headerName: 'Bond',
      disableColumnMenu: true,
      width: 175,
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" tooltipInfo="Node operator's share of stake." />,
      type: 'number',
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.mix_id}`}
        >
          {currencyToString({ amount: params.value, dp: 2 })}
        </MuiLink>
      ),
    },
    {
      field: 'profit_percentage',
      headerName: 'Profit Margin',
      disableColumnMenu: true,
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Profit Margin"
          tooltipInfo="Percentage of the delegators rewards that the operator takes as fee before rewards are distributed to the delegators."
        />
      ),
      headerClassName: 'MuiDataGrid-header-override',
      width: 160,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...getCellStyles(theme, params.row), textAlign: 'left' }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.mix_id}`}
        >
          {params.value}%
        </MuiLink>
      ),
    },
    {
      field: 'operating_cost',
      headerName: 'Operating Cost',
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Operating Cost"
          tooltipInfo="Monthly operational cost of running this node. This cost is set by the operator and it influences how the rewards are split between the operator and delegators."
        />
      ),
      headerClassName: 'MuiDataGrid-header-override',
      width: 170,
      headerAlign: 'left',
      disableColumnMenu: true,
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...getCellStyles(theme, params.row), textAlign: 'left' }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.mix_id}`}
        >
          {params.value} NYM
        </MuiLink>
      ),
    },
    {
      field: 'node_performance',
      headerName: 'Routing Score',
      disableColumnMenu: true,
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Routing Score"
          tooltipInfo="Mixnode's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test."
        />
      ),
      headerClassName: 'MuiDataGrid-header-override',
      width: 165,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...getCellStyles(theme, params.row), textAlign: 'left' }}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.mix_id}`}
        >
          {params.value}%
        </MuiLink>
      ),
    },
    {
      field: 'owner',
      headerName: 'Owner',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      width: 120,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          href={`${NYM_BIG_DIPPER}/account/${params.value}`}
          target="_blank"
          sx={getCellStyles(theme, params.row)}
          data-testid="big-dipper-link"
        >
          {splice(7, 29, params.value)}
        </MuiLink>
      ),
    },
    {
      field: 'location',
      headerName: 'Location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      disableColumnMenu: true,
      width: 120,
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
          <Tooltip text={params.value} id="mixnode-location-text">
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
      field: 'host',
      headerName: 'Host',
      renderHeader: () => <CustomColumnHeading headingTitle="Host" />,
      disableColumnMenu: true,
      headerClassName: 'MuiDataGrid-header-override',
      width: 130,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnode/${params.row.mix_id}`}
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
              withFilters
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

      {itemSelectedForDelegation && (
        <DelegateModal
          onClose={() => setItemSelectedForDelegation(undefined)}
          header="Delegate"
          buttonText="Delegate stake"
          denom="nym"
          onOk={(delegationModalProps: DelegationModalProps) => handleNewDelegation(delegationModalProps)}
          identityKey={itemSelectedForDelegation.identityKey}
          mixId={itemSelectedForDelegation.mixId}
        />
      )}

      {confirmationModalProps && (
        <DelegationModal
          {...confirmationModalProps}
          open={Boolean(confirmationModalProps)}
          onClose={async () => {
            setConfirmationModalProps(undefined);
            // await fetchBalance();
          }}
        />
      )}
    </>
  );
};
