import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button, Chip, Stack, Tooltip, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { Network } from 'src/types';
import { urls } from 'src/context';
import { NymCard } from 'src/components';
import { IdentityKey } from 'src/components/IdentityKey';
import { NodeStatus } from 'src/components/NodeStatus';
import { getIntervalAsDate } from 'src/utils';
import { UpgradeRounded } from '@mui/icons-material';
import { TBondedMixnode } from 'src/requests/mixnodeDetails';
import { Node as NodeIcon } from '../../svg-icons/node';
import { Cell, Header, NodeTable } from './NodeTable';
import { BondedMixnodeActions, TBondedMixnodeActions } from './BondedMixnodeActions';
import { NodeStats } from './NodeStats';
import { TBondedNymNode } from 'src/requests/nymNodeDetails';

const textWhenNotName = 'This node has not yet set a name';

const headers: Header[] = [
  {
    header: 'Stake',
    id: 'stake',
    sx: { pl: 0 },
  },
  {
    header: 'Bond',
    id: 'bond',
  },
  {
    header: 'Stake saturation',
    id: 'stake-saturation',
  },
  {
    header: 'PM',
    id: 'profit-margin',
    tooltipText:
      'The percentage of the node rewards that you as the node operator will take before the rest of the reward is shared between you and the delegators.',
  },
  {
    header: 'Operating cost',
    id: 'operator-cost',
    tooltipText:
      'Monthly operational costs of running your node. The cost also influences how the rewards are split between you and your delegators. ',
  },
  {
    header: 'Operator rewards',
    id: 'operator-rewards',
    tooltipText:
      'This is your (operator) rewards including the PM and cost. Rewards are automatically compounded every epoch. You can redeem your rewards at any time.',
  },
  {
    header: 'No. delegators',
    id: 'delegators',
  },
  {
    id: 'menu-button',
    sx: { width: 34, maxWidth: 34 },
  },
];

export const BondedMixnode = ({
  nymnode,
  network,
  onShowMigrateToNymNodeModal,
  onActionSelect,
}: {
  nymnode: TBondedNymNode;
  network?: Network;
  onShowMigrateToNymNodeModal: () => void;
  onActionSelect: (action: TBondedMixnodeActions) => void;
}) => {
  const [nextEpoch, setNextEpoch] = useState<string | Error>();
  const navigate = useNavigate();
  const {
    name,
    nodeId,
    stake,
    bond,
    stakeSaturation,
    profitMargin,
    operatorRewards,
    operatorCost,
    delegators,
    status,
    identityKey,
    host,
  } = nymnode;

  const getNextInterval = async () => {
    try {
      const { nextEpoch: newNextEpoch } = await getIntervalAsDate();
      setNextEpoch(newNextEpoch);
    } catch {
      setNextEpoch(Error());
    }
  };
  const cells: Cell[] = [
    {
      cell: `${stake.amount} ${stake.denom}`,
      id: 'stake-cell',
    },
    {
      cell: `${bond.amount} ${bond.denom}`,
      id: 'bond-cell',
    },
    {
      cell: `${stakeSaturation}%`,
      id: 'stake-saturation-cell',
    },
    {
      cell: `${profitMargin}%`,
      id: 'pm-cell',
    },
    {
      cell: operatorCost ? `${operatorCost.amount} ${operatorCost.denom}` : '-',
      id: 'operator-cost-cell',
    },
    {
      cell: operatorRewards ? `${operatorRewards.amount} ${operatorRewards.denom}` : '-',
      id: 'operator-rewards-cell',
    },
    {
      cell: delegators,
      id: 'delegators-cell',
    },
    {
      cell: nymnode.isUnbonding ? (
        <Chip label="Pending unbond" sx={{ textTransform: 'initial' }} />
      ) : (
        <BondedMixnodeActions
          onActionSelect={onActionSelect}
          disabledRedeemAndCompound={(operatorRewards && Number(operatorRewards.amount) === 0) || false}
        />
      ),
      id: 'actions-cell',
      align: 'right',
    },
  ];

  useEffect(() => {
    getNextInterval();
  }, []);

  return (
    <Stack gap={2}>
      <NymCard
        borderless
        title={
          <Stack gap={3}>
            <Box display="flex" alignItems="center" gap={2}>
              <Typography variant="h5" fontWeight={600}>
                Mix node
              </Typography>
              <NodeStatus status={status} />
            </Box>
            {name?.includes(textWhenNotName) ? null : (
              <Typography fontWeight="regular" variant="h6" width="fit-content">
                {name}
              </Typography>
            )}
            <Tooltip title={host} placement="top" arrow>
              <Box width="fit-content">
                <IdentityKey identityKey={identityKey} />
              </Box>
            </Tooltip>
          </Stack>
        }
        Action={
          <Box display="flex" flexDirection="column" alignItems="flex-end" justifyContent="space-between" height={70}>
            <Stack direction="row" gap={1}>
              <Tooltip
                title={
                  nymnode.isUnbonding
                    ? 'You have a pending unbond event. Node settings are disabled.'
                    : 'Node settings are disabled for legacy nodes. Please migrate your node in order to access your node settings.'
                }
              >
                <Box>
                  <Button
                    variant="text"
                    color="secondary"
                    onClick={() => navigate('/bonding/node-settings')}
                    startIcon={<NodeIcon />}
                    disabled
                  >
                    Node Settings
                  </Button>
                </Box>
              </Tooltip>
              <Button
                startIcon={<UpgradeRounded />}
                variant="contained"
                disableElevation
                onClick={onShowMigrateToNymNodeModal}
              >
                Migrate to Nym Node
              </Button>
            </Stack>

            {nextEpoch instanceof Error ? null : (
              <Typography fontSize={14} marginRight={1}>
                Next epoch starts at <b>{nextEpoch}</b>
              </Typography>
            )}
          </Box>
        }
      >
        <NodeTable headers={headers} cells={cells} />
        {network && (
          <Typography sx={{ mt: 2, fontSize: 'small' }}>
            Check more stats of your node on the{' '}
            <Link href={`${urls(network).networkExplorer}/network-components/mixnode/${nodeId}`} target="_blank">
              explorer
            </Link>
          </Typography>
        )}
      </NymCard>
      <NodeStats bondedNode={nymnode} />
    </Stack>
  );
};
