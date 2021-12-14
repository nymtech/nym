import React, { useContext, useState } from 'react'
import { Box, Dialog, Typography } from '@mui/material'
import { SettingsOutlined } from '@mui/icons-material'
import { NymCard } from '../../components'
import { ClientContext } from '../../context/main'
import { TabPanel, Tabs } from './tabs'
import { Profile } from './profile'
import { SystemVariables } from './system-variables'
import { NodeStats } from './node-stats'

const tabs = ['Profile', 'System variables', 'Node stats']

export const Settings = () => {
  const { showSettings, handleShowSettings } = useContext(ClientContext)
  const [selectedTab, setSelectedTab] = useState(0)

  const handleTabChange = (event: React.SyntheticEvent, newTab: number) => setSelectedTab(newTab)

  return showSettings ? (
    <Dialog open={true} onClose={handleShowSettings} maxWidth="md" fullWidth>
      <NymCard
        title={
          <Box display="flex" alignItems="center">
            <SettingsOutlined sx={{ mr: 1 }} /> Settings
          </Box>
        }
        noPadding
      >
        <>
          <Typography variant="h5" sx={{ py: 2, px: 4 }}>
            Node settings
          </Typography>
          <Tabs tabs={tabs} selectedTab={selectedTab} onChange={handleTabChange} />
          <TabPanel>
            {selectedTab === 0 && <Profile />}
            {selectedTab === 1 && <SystemVariables />}
            {selectedTab === 2 && <NodeStats />}
          </TabPanel>
        </>
      </NymCard>
    </Dialog>
  ) : null
}
