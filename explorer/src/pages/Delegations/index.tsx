import React, { useEffect } from 'react';
import { Alert, AlertTitle, Box, Button, Card, Chip, IconButton, Tooltip, Typography } from '@mui/material';
import { Link, useNavigate } from 'react-router-dom';
import { DelegationModal, DelegationModalProps, Title, UniversalDataGrid } from '@src/components';
import { useWalletContext } from '@src/context/wallet';
import { GridColDef } from '@mui/x-data-grid';
import { unymToNym } from '@src/utils/currency';
import {
  DelegationWithRewards,
  DelegationsProvider,
  PendingEvent,
  useDelegationsContext,
} from '@src/context/delegations';
import { urls } from '@src/utils';
import { useClipboard } from 'use-clipboard-copy';
import { Close } from '@mui/icons-material';

const mapToDelegationsRow = (delegation: DelegationWithRewards, index: number) => ({
  identity: delegation.identityKey,
  mix_id: delegation.mix_id,
  amount: `${unymToNym(delegation.amount.amount)} NYM`,
  rewards: `${unymToNym(delegation.rewards)} NYM`,
  id: index,
  pending: delegation.pending,
});

const Banner = ({ onClose }: { onClose: () => void }) => {
  const { copy } = useClipboard();

  return (
    <Alert
      severity="info"
      sx={{ mb: 3, fontSize: 'medium', width: '100%' }}
      action={
        <IconButton size="small" onClick={onClose}>
          <Close fontSize="small" />
        </IconButton>
      }
    >
      <AlertTitle> Mobile Delegations Beta</AlertTitle>
      <Box>
        <Typography>
          This is a beta release for mobile delegations If you have any feedback or feature suggestions contact us at
          support@nymte.ch
          <Button size="small" onClick={() => copy('support@nymte.ch')} sx={{ display: 'inline-block' }}>
            Copy
          </Button>
        </Typography>
      </Box>
    </Alert>
  );
};

const DelegationsPage = () => {
  const [confirmationModalProps, setConfirmationModalProps] = React.useState<DelegationModalProps | undefined>();
  const [isLoading, setIsLoading] = React.useState(false);
  const [showBanner, setShowBanner] = React.useState(true);

  const { isWalletConnected } = useWalletContext();
  const { handleGetDelegations, handleUndelegate, delegations } = useDelegationsContext();
  const navigate = useNavigate();

  useEffect(() => {
    let timeoutId: NodeJS.Timeout;

    const fetchDelegations = async () => {
      setIsLoading(true);
      try {
        await handleGetDelegations();
      } catch (error) {
        setConfirmationModalProps({
          status: 'error',
          message: "Couldn't fetch delegations. Please try again later.",
        });
      } finally {
        setIsLoading(false);

        timeoutId = setTimeout(() => {
          fetchDelegations();
        }, 60_000);
      }
    };

    fetchDelegations();

    return () => {
      clearTimeout(timeoutId);
    };
  }, [handleGetDelegations]);

  const getTooltipTitle = (pending: PendingEvent) => {
    if (pending?.kind === 'undelegate') {
      return 'You have an undelegation pending';
    }

    if (pending?.kind === 'delegate') {
      return `You have a delegation pending worth ${unymToNym(pending.amount.amount)} NYM`;
    }

    return undefined;
  };

  const onUndelegate = async (mixId: number) => {
    setConfirmationModalProps({ status: 'loading' });

    try {
      const tx = await handleUndelegate(mixId);

      if (tx) {
        setConfirmationModalProps({
          status: 'success',
          message: 'Undelegation can take up to one hour to process',
          transactions: [
            { url: `${urls('MAINNET').blockExplorer}/transaction/${tx.transactionHash}`, hash: tx.transactionHash },
          ],
        });
      }
    } catch (error) {
      if (error instanceof Error) {
        setConfirmationModalProps({ status: 'error', message: error.message });
      }
    }
  };

  const columns: GridColDef[] = [
    {
      field: 'identity',
      headerName: 'Identity Key',
      width: 400,
      disableColumnMenu: true,
      disableReorder: true,
      sortable: false,
      headerAlign: 'left',
    },
    {
      field: 'mix_id',
      headerName: 'Mix ID',
      width: 150,
      disableColumnMenu: true,
      disableReorder: true,
      sortable: false,
      headerAlign: 'left',
    },
    {
      field: 'amount',
      headerName: 'Amount',
      width: 150,
      disableColumnMenu: true,
      disableReorder: true,
      sortable: false,
      headerAlign: 'left',
    },
    {
      field: 'rewards',
      headerName: 'Rewards',
      width: 150,
      disableColumnMenu: true,
      disableReorder: true,
      sortable: false,
      headerAlign: 'left',
    },
    {
      field: 'undelegate',
      headerName: '',
      minWidth: 150,
      flex: 1,
      disableColumnMenu: true,
      disableReorder: true,
      sortable: false,
      headerAlign: 'right',
      renderCell: (params) => {
        const { pending } = params.row;

        return (
          <Box sx={{ width: '100%', display: 'flex', justifyContent: 'end' }}>
            {pending ? (
              <Tooltip
                placement="left"
                title={getTooltipTitle(pending as PendingEvent)}
                onClick={(e) => e.stopPropagation()}
                PopperProps={{}}
              >
                <Chip size="small" label="Pending events" />
              </Tooltip>
            ) : (
              <Button
                size="small"
                variant="outlined"
                onClick={(e) => {
                  e.stopPropagation();
                  onUndelegate(params.row.mix_id);
                }}
              >
                Undelegate
              </Button>
            )}
          </Box>
        );
      },
    },
  ];

  const handleRowClick = (params: any) => {
    navigate(`/network-components/mixnode/${params.row.mix_id}`);
  };

  return (
    <Box>
      {confirmationModalProps && (
        <DelegationModal
          {...confirmationModalProps}
          open={Boolean(confirmationModalProps)}
          onClose={async () => {
            if (confirmationModalProps.status === 'success') {
              await handleGetDelegations();
            }
            setConfirmationModalProps(undefined);
          }}
          sx={{
            width: {
              xs: '90%',
              sm: 600,
            },
          }}
        />
      )}
      {showBanner && <Banner onClose={() => setShowBanner(false)} />}
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Title text="Your Delegations" />
        <Button variant="contained" color="primary" component={Link} to="/network-components/mixnodes">
          Delegate
        </Button>
      </Box>
      {!isWalletConnected ? (
        <Box>
          <Typography mb={2} variant="h6">
            Connect your wallet to view your delegations.
          </Typography>
        </Box>
      ) : null}

      <Card
        sx={{
          mt: 2,
          padding: 2,
          height: '100%',
        }}
      >
        <UniversalDataGrid
          onRowClick={handleRowClick}
          rows={delegations?.map(mapToDelegationsRow) || []}
          columns={columns}
          loading={isLoading}
        />
      </Card>
    </Box>
  );
};

export const Delegations = () => (
  <DelegationsProvider>
    <DelegationsPage />
  </DelegationsProvider>
);
