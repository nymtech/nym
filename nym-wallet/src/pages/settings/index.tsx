import React, { useContext, useEffect, useState } from 'react';
import { Alert, Box, Dialog } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import { NymCard } from '../../components';
import { AppContext } from '../../context/main';
import { Tabs } from './tabs';
import { Profile } from './profile';
import { SystemVariables } from './system-variables';
import { NodeStats } from './node-stats';
import { useSettingsState } from './useSettingsState';
import { NodeStatus } from '../../components/NodeStatus';
import { Node as NodeIcon } from '../../svg-icons/node';

const tabs = ['Profile', 'System variables', 'Node stats'];

export const Settings = () => {
  const [selectedTab, setSelectedTab] = useState(0);

  const { mixnodeDetails, showSettings, getBondDetails, handleShowSettings } = useContext(AppContext);
  const { status, saturation, rewardEstimation, inclusionProbability, updateAllMixnodeStats } = useSettingsState();

  const handleTabChange = (_: React.SyntheticEvent, newTab: number) => setSelectedTab(newTab);

  useEffect(() => {
    getBondDetails();
    if (mixnodeDetails) {
      updateAllMixnodeStats(mixnodeDetails.mix_node.identity_key);
    }
  }, [showSettings, selectedTab]);

  return showSettings ? (
    <Dialog open onClose={handleShowSettings} maxWidth="md" fullWidth>
      <NymCard
        title={
          <Box width="100%" display="flex" justifyContent="space-between">
            <Box display="flex" alignItems="center">
              <NodeIcon sx={{ mr: 1 }} />
              Node Settings
            </Box>
            <CloseIcon onClick={handleShowSettings} cursor="pointer" />
          </Box>
        }
        Action={<NodeStatus status={status} />}
        dataTestid="node-settings"
        noPadding
      >
        <>
          <Tabs tabs={tabs} selectedTab={selectedTab} onChange={handleTabChange} disabled={!mixnodeDetails} />
          {!mixnodeDetails && (
            <Alert severity="info" sx={{ m: 4 }}>
              You do not currently have a node running
            </Alert>
          )}
          {selectedTab === 0 && <Profile />}
          {selectedTab === 1 && (
            <SystemVariables
              saturation={saturation}
              rewardEstimation={rewardEstimation}
              inclusionProbability={inclusionProbability}
            />
          )}
          {selectedTab === 2 && mixnodeDetails && <NodeStats mixnodeId={mixnodeDetails.mix_node.identity_key} />}
        </>
      </NymCard>
    </Dialog>
  ) : null;
};
