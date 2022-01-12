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
import { useSettingsState } from './useSettingsState'
import { NodeStatus } from '../../components/NodeStatus'

const tabs = ['Profile', 'System variables', 'Node stats']

export const Settings = () => {
  const { showSettings, handleShowSettings } = useContext(ClientContext)
  const [selectedTab, setSelectedTab] = useState(0)

  const { mixnodeDetails, status, saturation, rewardEstimation, getBondDetails } = useSettingsState(showSettings)

  const handleTabChange = (_: React.SyntheticEvent, newTab: number) => setSelectedTab(newTab)

  return showSettings ? (
    <Dialog open={true} onClose={handleShowSettings} maxWidth="md" fullWidth>
      <NymCard
        title={
          <Box display="flex" alignItems="center">
            <SettingsOutlined sx={{ mr: 1 }} />
            Node Settings
          </Box>
        }
        Action={<NodeStatus status={status} />}
        noPadding
      >
        <>
          <Tabs tabs={tabs} selectedTab={selectedTab} onChange={handleTabChange} disabled={!mixnodeDetails} />
          <Overview details={mixnodeDetails} />
          {!mixnodeDetails && (
            <Alert severity="info" sx={{ m: 4 }}>
              You don't currently have a node running
            </Alert>
          )}
          {selectedTab === 0 && mixnodeDetails && <Profile />}
          {selectedTab === 1 && mixnodeDetails && (
            <SystemVariables
              mixnodeDetails={mixnodeDetails.mix_node}
              saturation={saturation}
              rewardEstimation={rewardEstimation}
              onUpdate={getBondDetails}
            />
          )}
          {selectedTab === 2 && mixnodeDetails && <NodeStats mixnodeId={mixnodeDetails.mix_node.identity_key} />}
        </>
      </NymCard>
    </Dialog>
  ) : null
}
