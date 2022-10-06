import React, { useContext, useState } from 'react';
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

import { NodeGeneralSettings } from './settings-pages/general-settings';
import { NodeUnbondPage } from './settings-pages/NodeUnbondPage';
import { nodeSettingsNav } from './node-settings.constant';

const useQuery = () => {
  const { search } = useLocation();

  return React.useMemo(() => new URLSearchParams(search), [search]);
};

const getTabs = () => {
  const tabs = Object.values(nodeSettingsNav) as string[];
  const length = tabs.length;
  return tabs.splice(0, length / 2);
};

export const NodeSettings = () => {
  const theme = useTheme();
  const { network } = useContext(AppContext);
  const { bondedNode, unbond, isLoading } = useBondingContext();
  const navigate = useNavigate();
  const query = useQuery();
  const queryTab = query.get('tab');
  const tabs = getTabs();

  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps>();
  const [value, setValue] = React.useState(
    queryTab === 'unbond' ? nodeSettingsNav['Unbond'] : nodeSettingsNav['General'],
  );
  const handleChange = (event: React.SyntheticEvent, tab: number) => {
    setValue(tab);
  };

  const handleUnbond = async (fee?: FeeDetails) => {
    const tx = await unbond(fee);
    setConfirmationDetails({
      status: 'success',
      title: 'Unbond successful',
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
                  Node Settings
                </Typography>
              </Box>
            </Box>
            <Box sx={{ width: '100%' }}>
              <Tabs
                tabs={tabs}
                selectedTab={value}
                onChange={handleChange}
                tabSx={{
                  bgcolor: 'transparent',
                  borderBottom: 'none',
                  borderTop: 'none',
                  '& button': {
                    p: 0,
                    mr: 4,
                    minWidth: 'none',
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
        {nodeSettingsNav[value] === 'General' && bondedNode && <NodeGeneralSettings bondedNode={bondedNode} />}
        {nodeSettingsNav[value] === 'Unbond' && bondedNode && (
          <NodeUnbondPage bondedNode={bondedNode} onConfirm={handleUnbond} onError={handleError} />
        )}
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
          />
        )}
        {isLoading && <LoadingModal />}
      </NymCard>
    </PageLayout>
  );
};

export const NodeSettingsPage = () => (
  <BondingContextProvider>
    <NodeSettings />
  </BondingContextProvider>
);
