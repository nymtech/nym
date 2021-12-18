import React, { useContext, useEffect, useState } from 'react'
import { Alert, Box, Dialog, Typography } from '@mui/material'
import { SettingsOutlined } from '@mui/icons-material'
import { NymCard } from '../../components'
import { ClientContext } from '../../context/main'
import { Tabs } from './tabs'
import { Profile } from './profile'
import { SystemVariables } from './system-variables'
import { NodeStats } from './node-stats'
import { Overview } from './overview'
import { getMixnodeBondDetails } from '../../requests'
import { TMixnodeBondDetails } from '../../types'

const tabs = ['Profile', 'System variables', 'Node stats']

export const Settings = () => {
  const { showSettings, handleShowSettings } = useContext(ClientContext)
  const [selectedTab, setSelectedTab] = useState(0)
  const [mixnodeDetails, setMixnodeDetails] = useState<TMixnodeBondDetails | null>()

  useEffect(() => {
    const getBondDetails = async () => {
      const details = await getMixnodeBondDetails()
      setMixnodeDetails(details)
    }
    if (showSettings) getBondDetails()
    if (!showSettings) setSelectedTab(0)
  }, [showSettings])

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
          <Tabs tabs={tabs} selectedTab={selectedTab} onChange={handleTabChange} disabled={!mixnodeDetails} />
          <Overview details={mixnodeDetails} />
          {!mixnodeDetails && (
            <Alert severity="info" sx={{ m: 4 }}>
              You don't currently have a node running
            </Alert>
          )}
          {selectedTab === 0 && mixnodeDetails && <Profile />}
          {selectedTab === 1 && mixnodeDetails && (
            <SystemVariables mixnodeDetails={mixnodeDetails.mix_node} pledge={mixnodeDetails.pledge_amount} />
          )}
          {selectedTab === 2 && mixnodeDetails && <NodeStats mixnodeId={mixnodeDetails.mix_node.identity_key} />}
        </>
      </NymCard>
    </Dialog>
  ) : null
}
