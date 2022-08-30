import React, { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FeeDetails } from '@nymproject/types';
import { Box, Typography, Stack, Toolbar, Button, Divider } from '@mui/material';
import { Close } from '@mui/icons-material';
import { ConfirmationDetailProps, ConfirmationDetailsModal } from 'src/components/Bonding/modals/ConfirmationModal';
import { Node as NodeIcon } from 'src/svg-icons/node';
import { NymCard } from '../../../components';
import { PageLayout } from '../../../layouts';
import { useBondingContext, TBondedMixnode } from '../../../context';
import { AppContext, urls } from 'src/context/main';

import { NodeGeneralSettings } from './general-settings';
import { UnbondModal } from '../../../components/Bonding/modals/UnbondModal';

const nodeSettingsNav = ['General', 'Unbond'];

// TODO: remove commented code to emulate a bonded mixnode
const bondedMixnodeMock: TBondedMixnode = {
  name: 'Monster node',
  identityKey: '7mjM2fYbtN6kxMwp1TrmQ4VwPks3URR5pBgWPWhzT98F',
  stake: { denom: 'nym', amount: '1234' },
  bond: { denom: 'nym', amount: '1234' },
  stakeSaturation: 95,
  profitMargin: 15,
  operatorRewards: { denom: 'nym', amount: '1234' },
  delegators: 5423,
  status: 'active',
};

export const NodeSettingsPage = () => {
  const [settingsCard, setSettingsCard] = useState<string>(nodeSettingsNav[0]);
  const [confirmationDetails, setConfirmationDetails] = useState<ConfirmationDetailProps>();

  const {
    network,
    userBalance: { originalVesting },
  } = useContext(AppContext);

  const { bondedNode, unbond } = useBondingContext();

  const navigate = useNavigate();

  const handleUnbond = async (fee?: FeeDetails) => {
    navigate('/bonding');
    const tx = await unbond(fee);
    setConfirmationDetails({
      status: 'success',
      title: 'Unbond successful',
      txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
    });
  };

  const handleError = (error: string) => {
    navigate('/bonding');
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
            <Toolbar disableGutters sx={{ minHeight: 'fit-content' }}>
              {nodeSettingsNav.map((item) => (
                <Button
                  size="small"
                  sx={{
                    p: 0,
                    mr: 4,
                    fontWeight: 600,
                    color: settingsCard === item ? 'primary.main' : '#B9B9B9',
                    textDecoration: settingsCard === item ? 'underline' : 'none',
                    textDecorationThickness: '4px',
                    textUnderlineOffset: '4px',
                    justifyContent: 'start',
                    ':hover': {
                      bgcolor: 'transparent',
                      color: 'primary.main',
                    },
                  }}
                  onClick={() => setSettingsCard(item)}
                >
                  {item}
                </Button>
              ))}
            </Toolbar>
          </Stack>
        }
        Action={
          <Button
            size="small"
            sx={{
              color: 'text.primary',
            }}
            onClick={() => navigate('/bonding')}
            startIcon={<Close />}
          ></Button>
        }
      >
        <Divider />
        {settingsCard === nodeSettingsNav[0] && (
          <NodeGeneralSettings onSaveChanges={() => console.log('save changes')} />
        )}
        {settingsCard === nodeSettingsNav[1] && bondedMixnodeMock && (
          <UnbondModal
            node={bondedMixnodeMock}
            onClose={() => setSettingsCard(nodeSettingsNav[0])}
            onConfirm={handleUnbond}
            onError={handleError}
          />
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
