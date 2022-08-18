import React, { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Typography, Stack, IconButton, Toolbar, Button, Divider } from '@mui/material';
import { Close } from '@mui/icons-material';
import { Node as NodeIcon } from 'src/svg-icons/node';
import { NymCard } from '../../../components';
import { PageLayout } from '../../../layouts';
// import { AppContext } from '../../context/main';

import { NodeGeneralSettings } from './general-settings';
// import { NodeParametersCard } from './InfoSettings';
// import { UnbondModal } from '../../modals';

const nodeSettingsNav = ['General', 'Unbond'];

export const NodeSettingsPage = () => {
  const [settingsCard, setSettingsCard] = useState<string>(nodeSettingsNav[0]);

  const navigate = useNavigate();

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
              <IconButton
                onClick={() => navigate('/bonding')}
                size="small"
                sx={{
                  color: 'text.primary',
                }}
              >
                <Close />
              </IconButton>
            </Box>
            <Toolbar disableGutters sx={{ height: '30px' }}>
              {nodeSettingsNav.map((item) => (
                <Button size="small" sx={{ p: 0, mr: 2, color: 'inherit' }} onClick={() => setSettingsCard(item)}>
                  {item}
                </Button>
              ))}
            </Toolbar>
          </Stack>
        }
      >
        <Divider />
        {settingsCard === nodeSettingsNav[0] && (
          <NodeGeneralSettings onSaveChanges={() => console.log('save changes')} />
        )}
        {settingsCard === nodeSettingsNav[1] && (
          <NodeGeneralSettings onSaveChanges={() => console.log('save changes')} />
        )}
      </NymCard>
    </PageLayout>
  );
};
