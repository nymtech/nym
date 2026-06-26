import React, { useCallback, useContext, useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { FeeDetails } from '@nymproject/types';
import { Box, Typography, Stack, IconButton, Divider } from '@mui/material';
import { Close } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import { Node as NodeIcon } from 'src/svg-icons/node';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { NymCard } from 'src/components';
import { PageLayout } from 'src/layouts';
import { Tabs } from 'src/components/Tabs';
import { useBondingContext, BondingContextProvider } from 'src/context';
import { AppContext, urls } from 'src/context/main';

import { getIntervalAsDate } from 'src/utils';
import { NodeGeneralSettings } from './settings-pages/general-settings';
import { NodeUnbondPage } from './settings-pages/NodeUnbondPage';
import { NodeCostParametersPage } from './settings-pages/NodeCostParameters';
import { NavItems, makeNavItems } from './node-settings.constant';

export const NodeSettings = () => {
  const theme = useTheme();
  const { network } = useContext(AppContext);
  const { bondedNode, unbond, isLoading, error, refresh } = useBondingContext();
  const navigate = useNavigate();
  const location = useLocation();

  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps | undefined>();
  const [value, setValue] = React.useState<NavItems>('General');

  const handleError = useCallback(
    (errorMessage: string) => {
      refresh();
      setConfirmationDetails({
        status: 'error',
        title: 'An error occurred',
        subtitle: errorMessage,
      });
    },
    [refresh],
  );

  const handleChange = (_: React.SyntheticEvent, tab: string) => {
    setValue(tab as NavItems);
  };

  useEffect(() => {
    if (location.state === 'unbond') {
      setValue('Unbond');
    }
    if (location.state === 'cost-parameters') {
      setValue('Cost Parameters');
    }
  }, [location]);

  const handleUnbond = async (fee?: FeeDetails) => {
    const tx = await unbond(fee);
    if (!tx) {
      return;
    }
    const { nextEpoch } = await getIntervalAsDate();
    setConfirmationDetails({
      status: 'success',
      title: 'Unbond successful',
      subtitle: `This operation will complete when the new epoch starts at: ${nextEpoch}`,
      txUrl: `${urls(network).blockExplorer}/tx/${tx.transaction_hash}`,
    });
  };

  const handleUpdateCostParameters = async (txHash?: string) => {
    setConfirmationDetails({
      status: 'success',
      title: 'Cost Parameters Updated',
      subtitle: 'Your cost parameters have been successfully updated',
      txUrl: txHash ? `${urls(network).blockExplorer}/tx/${txHash}` : undefined,
    });
  };

  const showBondingError = error && !confirmationDetails;
  if (isLoading) return <LoadingModal />;

  if (!bondedNode) {
    return null;
  }

  return (
    <PageLayout>
      <NymCard
        borderless
        noPadding
        title={
          <Stack gap={2} sx={{ py: 0 }}>
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
              }}
            >
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <NodeIcon />
                <Typography variant="h6" fontWeight={600}>
                  Nym Node Settings
                </Typography>
              </Box>
            </Box>
            <Box sx={{ width: '100%' }}>
              <Tabs
                tabs={makeNavItems(bondedNode)}
                selectedTab={value}
                onChange={handleChange}
                tabSx={{
                  bgcolor: 'transparent',
                  borderBottom: 'none',
                  borderTop: 'none',
                  '& button': {
                    p: 0,
                    mr: 4,
                    fontSize: 16,
                  },
                  '& button:hover': {
                    color: theme.palette.nym.highlight,
                    opacity: 1,
                  },
                }}
                tabIndicatorStyles={{ height: 4, bottom: '6px', borderRadius: '2px' }}
              />
            </Box>
          </Stack>
        }
        Action={
          <IconButton
            size="small"
            sx={{
              color: 'text.primary',
            }}
            onClick={() => navigate('/bonding')}
          >
            <Close />
          </IconButton>
        }
      >
        <Divider />
        {value === 'General' && bondedNode && <NodeGeneralSettings bondedNode={bondedNode} />}
        {value === 'Cost Parameters' && bondedNode && (
          <NodeCostParametersPage
            bondedNode={bondedNode}
            onConfirm={handleUpdateCostParameters}
            onError={handleError}
          />
        )}
        {value === 'Unbond' && bondedNode && (
          <NodeUnbondPage bondedNode={bondedNode} onConfirm={handleUnbond} onError={handleError} />
        )}
        {confirmationDetails && (
          <ConfirmationDetailsModal
            title={confirmationDetails.title}
            subtitle={confirmationDetails.subtitle || 'This operation can take up to one hour to process'}
            status={confirmationDetails.status}
            txUrl={confirmationDetails.txUrl}
            onClose={() => {
              const wasSuccess = confirmationDetails.status === 'success';
              setConfirmationDetails(undefined);
              if (wasSuccess) {
                navigate('/bonding');
              } else {
                refresh();
              }
            }}
          >
            {confirmationDetails.status === 'success' && value === 'Unbond' && (
              <Typography fontWeight="bold">
                You should NOT shutdown your node until the unbond process is complete
              </Typography>
            )}
          </ConfirmationDetailsModal>
        )}
        {showBondingError && (
          <ErrorModal
            open
            title="An error occurred, please check logs for details"
            message={error}
            onClose={() => refresh()}
          />
        )}
      </NymCard>
    </PageLayout>
  );
};

export const NodeSettingsPage = () => (
  <BondingContextProvider>
    <NodeSettings />
  </BondingContextProvider>
);
