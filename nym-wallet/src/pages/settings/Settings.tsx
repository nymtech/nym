import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Divider, IconButton, Stack, Typography } from '@mui/material';
import { Close, SettingsOutlined } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { NymCard } from '../../components';
import { PageLayout } from '../../layouts';
import { Tabs } from '../../components/Tabs';
import GeneralSettings from './GeneralSettings';
import AdvancedSettings from './AdvancedSettings';
import SecuritySettings from './SecuritySettings';

const tabs = ['General', 'Security', 'Advanced'] as const;
type SettingsTabs = (typeof tabs)[number];

const Settings = () => {
  const [currentTab, setCurrentTab] = React.useState<SettingsTabs>('General');

  const navigate = useNavigate();
  const theme = useTheme();

  return (
    <PageLayout>
      <Box sx={{ mb: 2 }}>
        <Typography variant="overline" sx={{ color: 'text.secondary', letterSpacing: 1.2 }}>
          Preferences
        </Typography>
        <Typography variant="h4" sx={{ mt: 0.5 }}>
          Wallet settings
        </Typography>
      </Box>
      <NymCard
        borderless
        noPadding
        sx={{
          borderRadius: 4,
          backgroundColor: 'background.paper',
        }}
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
                <SettingsOutlined />
                <Typography variant="h6" fontWeight={600}>
                  Settings
                </Typography>
              </Box>
            </Box>
            <Box sx={{ width: '100%' }}>
              <Tabs
                tabs={tabs}
                selectedTab={currentTab}
                onChange={(e, tab) => setCurrentTab(tab as SettingsTabs)}
                tabSx={{
                  bgcolor: 'transparent',
                  borderBottom: 'none',
                  borderTop: 'none',
                  '& button': {
                    px: 2,
                    py: 1,
                    mr: 1.5,
                    minWidth: 'none',
                    fontSize: 15,
                    borderRadius: '999px',
                  },
                  '& button:hover': {
                    color: theme.palette.nym.highlight,
                    opacity: 1,
                  },
                }}
                tabIndicatorStyles={{
                  height: 3,
                  borderRadius: 3,
                  backgroundColor: theme.palette.primary.main,
                }}
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
            onClick={() => navigate('/balance')}
          >
            <Close />
          </IconButton>
        }
      >
        <Divider />
        {currentTab === 'General' && <GeneralSettings />}
        {currentTab === 'Security' && <SecuritySettings />}
        {currentTab === 'Advanced' && <AdvancedSettings />}
      </NymCard>
    </PageLayout>
  );
};

export default Settings;
