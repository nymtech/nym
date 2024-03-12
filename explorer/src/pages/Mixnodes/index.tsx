import * as React from 'react';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { Stack, Card, Grid, Box, Button } from '@mui/material';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { SelectChangeEvent } from '@mui/material/Select';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { useMainContext } from '@src/context/main';
import {
  DelegateIconButton,
  DelegationModal,
  DelegationModalProps,
  DelegateModal,
  CustomColumnHeading,
  StyledLink,
  Title,
  UniversalDataGrid,
  TableToolbar,
  Tooltip,
  MixNodeStatusDropdown,
  mixnodeToGridRow,
} from '@src/components';
import { MixNodeResponse, MixnodeStatusWithAll, toMixnodeStatus } from '@src/typeDefs/explorer-api';
import { NYM_BIG_DIPPER } from '@src/api/constants';
import { currencyToString } from '@src/utils/currency';
import { splice } from '@src/utils';
import { useGetMixNodeStatusColor, useIsMobile } from '@src/hooks';
import { useWalletContext } from '@src/context/wallet';
import { DelegationsProvider } from '@src/context/delegations';

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

  const navigate = useNavigate();
  const { isWalletConnected } = useWalletContext();
  const isMobile = useIsMobile();

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

  const handleOnDelegate = ({ identityKey, mixId }: { identityKey: string; mixId: number }) => {
    if (!isWalletConnected) {
      setConfirmationModalProps({
        status: 'info',
        message: 'Please connect your wallet to delegate',
      });
    } else {
      setItemSelectedForDelegation({ identityKey, mixId });
    }
  };

  const columns: GridColDef[] = [
    {
      field: 'delegate',
      disableColumnMenu: true,
      disableReorder: true,
      sortable: false,
      width: isMobile ? 25 : 100,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => null,
      renderCell: (params: GridRenderCellParams) => (
        <DelegateIconButton
          size="small"
          onDelegate={() => handleOnDelegate({ identityKey: params.row.identity_key, mixId: params.row.mix_id })}
        />
      ),
    },
    {
      field: 'identity_key',
      width: 325,
      headerName: 'Identity Key',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      renderCell: (params: GridRenderCellParams) => (
        <Stack direction="row" alignItems="center" gap={1}>
          <CopyToClipboard
            sx={{ mr: 0.5, color: 'grey.400' }}
            smallIcons
            value={params.value}
            tooltip={`Copy identity key ${params.value} to clipboard`}
          />
          <StyledLink
            to={`/network-components/mixnode/${params.row.mix_id}`}
            color={useGetMixNodeStatusColor(params.row.status)}
            dataTestId="identity-link"
          >
            {splice(7, 29, params.value)}
          </StyledLink>
        </Stack>
      ),
    },
    {
      field: 'mix_id',
      width: 85,
      align: 'center',
      hide: true,
      headerName: 'Mix ID',
      headerAlign: 'center',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Mix ID" />,
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/mixnode/${params.value}`}
          color={useGetMixNodeStatusColor(params.row.status)}
          data-testid="mix-id"
        >
          {params.value}
        </StyledLink>
      ),
    },

    {
      field: 'bond',
      width: 150,
      align: 'left',
      type: 'number',
      disableColumnMenu: true,
      headerName: 'Stake',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle="Stake" />,
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/mixnode/${params.row.mix_id}`}
          color={useGetMixNodeStatusColor(params.row.status)}
        >
          {currencyToString({ amount: params.value })}
        </StyledLink>
      ),
    },
    {
      field: 'stake_saturation',
      width: 185,
      align: 'center',
      headerName: 'Stake Saturation',
      headerAlign: 'center',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Stake Saturation"
          tooltipInfo="Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is 940k NYMs, computed as S/K where S is target amount of tokens staked in the network and K is the number of nodes in the reward set."
        />
      ),
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/mixnode/${params.row.mix_id}`}
          color={useGetMixNodeStatusColor(params.row.status)}
        >{`${params.value} %`}</StyledLink>
      ),
    },
    {
      field: 'pledge_amount',
      width: 150,
      align: 'left',
      type: 'number',
      headerName: 'Bond',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" tooltipInfo="Node operator's share of stake." />,
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/mixnode/${params.row.mix_id}`}
          color={useGetMixNodeStatusColor(params.row.status)}
        >
          {currencyToString({ amount: params.value })}
        </StyledLink>
      ),
    },
    {
      field: 'profit_percentage',
      width: 145,
      align: 'center',
      headerName: 'Profit Margin',
      headerAlign: 'center',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Profit Margin"
          tooltipInfo="Percentage of the delegators rewards that the operator takes as fee before rewards are distributed to the delegators."
        />
      ),
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/mixnode/${params.row.mix_id}`}
          color={useGetMixNodeStatusColor(params.row.status)}
        >{`${params.value}%`}</StyledLink>
      ),
    },
    {
      field: 'operating_cost',
      width: 170,
      align: 'center',
      headerName: 'Operating Cost',
      headerAlign: 'center',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Operating Cost"
          tooltipInfo="Monthly operational cost of running this node. This cost is set by the operator and it influences how the rewards are split between the operator and delegators."
        />
      ),
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/mixnode/${params.row.mix_id}`}
          color={useGetMixNodeStatusColor(params.row.status)}
        >{`${params.value} NYM`}</StyledLink>
      ),
    },
    {
      field: 'node_performance',
      width: 165,
      align: 'center',
      headerName: 'Routing Score',
      headerAlign: 'center',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => (
        <CustomColumnHeading
          headingTitle="Routing Score"
          tooltipInfo="Mixnode's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test."
        />
      ),
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/mixnode/${params.row.mix_id}`}
          color={useGetMixNodeStatusColor(params.row.status)}
        >{`${params.value}%`}</StyledLink>
      ),
    },
    {
      field: 'owner',
      width: 120,
      headerName: 'Owner',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`${NYM_BIG_DIPPER}/account/${params.value}`}
          color={useGetMixNodeStatusColor(params.row.status)}
          target="_blank"
          data-testid="big-dipper-link"
        >
          {splice(7, 29, params.value)}
        </StyledLink>
      ),
    },
    {
      field: 'location',
      width: 150,
      headerName: 'Location',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      renderCell: (params: GridRenderCellParams) => (
        <Tooltip text={params.value} id="mixnode-location-text">
          <Box
            sx={{
              overflow: 'hidden',
              whiteSpace: 'nowrap',
              textOverflow: 'ellipsis',
              cursor: 'pointer',
              color: useGetMixNodeStatusColor(params.row.status),
            }}
            onClick={() => handleSearch(params.value)}
          >
            {params.value}
          </Box>
        </Tooltip>
      ),
    },
    {
      field: 'host',
      width: 130,
      headerName: 'Host',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      disableColumnMenu: true,
      renderHeader: () => <CustomColumnHeading headingTitle="Host" />,
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          color={useGetMixNodeStatusColor(params.row.status)}
          to={`/network-components/mixnode/${params.row.mix_id}`}
        >
          {params.value}
        </StyledLink>
      ),
    },
  ];

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  return (
    <DelegationsProvider>
      <Box mb={2}>
        <Title text="Mixnodes" />
      </Box>
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
              childrenAfter={
                isWalletConnected && (
                  <Button fullWidth size="large" variant="outlined" color="primary" component={Link} to="/delegations">
                    Delegations
                  </Button>
                )
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
          onClose={() => {
            setItemSelectedForDelegation(undefined);
          }}
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
            if (confirmationModalProps.status === 'success') {
              navigate('/delegations');
            }
          }}
        />
      )}
    </DelegationsProvider>
  );
};
