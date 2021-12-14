import React, { useContext, useState } from 'react'
import { Box, Dialog, Slide, Tab, Tabs, Typography } from '@mui/material'
import { SettingsOutlined } from '@mui/icons-material'
import { NymCard } from '../../components'
import { ClientContext } from '../../context/main'

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
          <Tabs
            value={selectedTab}
            onChange={handleTabChange}
            aria-label="basic tabs example"
            sx={{ bgcolor: 'grey.200', borderTop: '1px solid', borderBottom: '1px solid', borderColor: 'grey.300' }}
            textColor="inherit"
          >
            {tabs.map((tabName, index) => (
              <Tab key={index} label={tabName} sx={{ textTransform: 'capitalize' }} />
            ))}
          </Tabs>
        </>
      </NymCard>
    </Dialog>
  ) : null
}
