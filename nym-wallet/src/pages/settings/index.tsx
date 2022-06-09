import React, { useContext, useEffect, useState } from 'react';
import { Alert, Box, Dialog } from '@mui/material';
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

  const { mixnodeDetails, getBondDetails } = useContext(AppContext);
  const { status, saturation, rewardEstimation, inclusionProbability } = useSettingsState(false);

  const handleTabChange = (_: React.SyntheticEvent, newTab: number) => setSelectedTab(newTab);

  return (
    <Dialog open maxWidth="md" fullWidth>
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
  );
};
