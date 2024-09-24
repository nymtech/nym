import React, { useContext, useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { FeeDetails } from '@nymproject/types';
import { Box, Typography, Stack, IconButton, Divider } from '@mui/material';
import { Close } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { Node as NodeIcon } from 'src/svg-icons/node';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { NymCard } from 'src/components';
import { PageLayout } from 'src/layouts';
import { Tabs } from 'src/components/Tabs';
import { useBondingContext, BondingContextProvider } from 'src/context';
import { AppContext, urls } from 'src/context/main';

import { isMixnode } from 'src/types';
import { getIntervalAsDate } from 'src/utils';
import { TBondedMixnode } from 'src/requests/mixnodeDetails';
import { NodeGeneralSettings } from './settings-pages/general-settings';
import { NodeUnbondPage } from './settings-pages/NodeUnbondPage';
import { NavItems, makeNavItems } from './node-settings.constant';
import { ApyPlayground } from './apy-playground';
import { NodeTestPage } from './node-test';

export const NodeSettings = () => {
  const theme = useTheme();
  const { network } = useContext(AppContext);
  const { bondedNode, unbond, isLoading } = useBondingContext();
  const navigate = useNavigate();
  const location = useLocation();

  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps | undefined>();
  const [value, setValue] = React.useState<NavItems>('General');

  const handleChange = (_: React.SyntheticEvent, tab: string) => {
    setValue(tab as NavItems);
  };

  useEffect(() => {
    if (location.state === 'unbond') {
      setValue('Unbond');
    }
    if (location.state === 'test-node') {
      setValue('Test my node');
    }
  }, [location]);

  const handleUnbond = async (fee?: FeeDetails) => {
    const tx = await unbond(fee);
    const { nextEpoch } = await getIntervalAsDate();
    setConfirmationDetails({
      status: 'success',
      title: 'Unbond successful',
      subtitle: `This operation will complete when the new epoch starts at: ${nextEpoch}`,
      txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    });
  };

  const handleError = (error: string) => {
    setConfirmationDetails({
      status: 'error',
      title: 'An error occurred',
      subtitle: error,
    });
  };

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
                  {isMixnode(bondedNode) ? 'Node' : 'Gateway'} Settings
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
        {value === 'Test my node' && <NodeTestPage />}
        {value === 'Unbond' && bondedNode && (
          <NodeUnbondPage bondedNode={bondedNode} onConfirm={handleUnbond} onError={handleError} />
        )}
        {value === 'Playground' && bondedNode && <ApyPlayground bondedNode={bondedNode as TBondedMixnode} />}
        {confirmationDetails && confirmationDetails.status === 'success' && (
          <ConfirmationDetailsModal
            title={confirmationDetails.title}
            subtitle={confirmationDetails.subtitle || 'This operation can take up to one hour to process'}
            status={confirmationDetails.status}
            txUrl={confirmationDetails.txUrl}
            onClose={() => {
              setConfirmationDetails(undefined);
              navigate('/bonding');
            }}
          >
            <Typography fontWeight="bold">
              You should NOT shutdown your {isMixnode(bondedNode) ? 'mix node' : 'gateway'} until the unbond process is
              complete
            </Typography>
          </ConfirmationDetailsModal>
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
