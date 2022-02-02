import React, { useContext, useState } from 'react'
import { Alert, Box, Dialog } from '@mui/material'
import { NymCard } from '../../components'
import { ClientContext } from '../../context/main'
import { Tabs } from './tabs'
import { Profile } from './profile'
import { SystemVariables } from './system-variables'
import { NodeStats } from './node-stats'
import { useSettingsState } from './useSettingsState'
import { NodeStatus } from '../../components/NodeStatus'
import { Node as NodeIcon } from '../../svg-icons/node'

const tabs = ['Profile', 'System variables', 'Node stats']

export const Settings = () => {
  const [selectedTab, setSelectedTab] = useState(0)

  const { mixnodeDetails, showSettings, handleShowSettings, getBondDetails } = useContext(ClientContext)
  const { status, saturation, rewardEstimation, inclusionProbability } = useSettingsState(showSettings)

  const handleTabChange = (_: React.SyntheticEvent, newTab: number) => setSelectedTab(newTab)

  return showSettings ? (
    <Dialog open={true} onClose={handleShowSettings} maxWidth="md" fullWidth>
      <NymCard
        title={
          <Box display="flex" alignItems="center">
            <NodeIcon sx={{ mr: 1 }} />
            Node Settings
          </Box>
        }
        Action={<NodeStatus status={status} />}
        noPadding
      >
        <>
          <Tabs tabs={tabs} selectedTab={selectedTab} onChange={handleTabChange} disabled={!mixnodeDetails} />
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
              inclusionProbability={inclusionProbability}
            />
          )}
          {selectedTab === 2 && mixnodeDetails && <NodeStats mixnodeId={mixnodeDetails.mix_node.identity_key} />}
        </>
      </NymCard>
    </Dialog>
  ) : null
}
