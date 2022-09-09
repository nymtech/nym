import React, { useContext, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FeeDetails } from '@nymproject/types';
import { Box, Typography, Stack, Button, Divider, IconButton } from '@mui/material';
import { Close } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { Node as NodeIcon } from 'src/svg-icons/node';
import { NymCard } from '../../../components';
import { PageLayout } from '../../../layouts';
import { Tabs } from 'src/components/Tabs';
import { useBondingContext, BondingContextProvider } from '../../../context';
import { AppContext, urls } from 'src/context/main';
import { NodeGeneralSettings } from './general-settings';
import { UnbondModal } from '../../../components/Bonding/modals/UnbondModal';
import { nodeSettingsNav } from './node-settings.constant';
import { TestNode } from 'src/pages/test-my-node';

export const NodeSettings = () => {
  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps>();
  const [value, setValue] = React.useState(0);

  const theme = useTheme();

  const handleChange = (event: React.SyntheticEvent, newValue: number) => {
    setValue(newValue);
  };

  const { network } = useContext(AppContext);

  const { bondedNode, unbond } = useBondingContext();

  const navigate = useNavigate();

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
                tabs={nodeSettingsNav}
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
          <IconButton onClick={() => navigate('/bonding')}>
            <Close />
          </IconButton>
        }
      >
        <Divider />
        {value === 0 && bondedNode && <NodeGeneralSettings bondedNode={bondedNode} />}
        {value === 1 && bondedNode && (
          <Box sx={{ p: 2 }}>
            <TestNode />
          </Box>
        )}
        {value === 2 && bondedNode && (
          <UnbondModal node={bondedNode} onClose={() => setValue(0)} onConfirm={handleUnbond} onError={handleError} />
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
      </NymCard>
    </PageLayout>
  );
};

export const NodeSettingsPage = () => (
  <BondingContextProvider>
    <NodeSettings />
  </BondingContextProvider>
);
