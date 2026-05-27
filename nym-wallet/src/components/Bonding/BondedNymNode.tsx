import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Box, Button, Chip, Stack, Tooltip, Typography } from '@mui/material';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';
import { Network } from 'src/types';
import { urls } from 'src/context';
import { NymCard } from 'src/components';
import { IdentityKey } from 'src/components/IdentityKey';
import { getIntervalAsDate } from 'src/utils';
import { TBondedNymNode } from 'src/requests/nymNodeDetails';
import { Node as NodeIcon } from '../../svg-icons/node';
import { Cell, Header, NodeTable } from './NodeTable';
import { BondedNymNodeActions, TBondedNymNodeActions } from './BondedNymNodeActions';
import { NodeOperatorInsights, type NodeStatusMetadata } from './NodeOperatorInsights';

const textWhenNotName = 'This node has not yet set a name';

/** Wallet default name vs legacy copy in BondedNymNode. */
function isUnsetNodeName(name: string | undefined): boolean {
  if (!name) {
    return true;
  }
  return (
    name.includes('Name has not been set') ||
    name.includes(textWhenNotName) ||
    name.toLowerCase().includes('not been set')
  );
}

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

export const BondedNymNode = ({
  nymnode,
  network,
  onActionSelect,
}: {
  nymnode: TBondedNymNode;
  network?: Network;
  onActionSelect: (action: TBondedNymNodeActions) => void;
}) => {
  const [nextEpoch, setNextEpoch] = useState<string | Error>();
  const [statusMeta, setStatusMeta] = useState<NodeStatusMetadata | null>(null);
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
    identityKey,
    host,
    uptime,
  } = nymnode;

  const handleStatusLoaded = useCallback((meta: NodeStatusMetadata) => {
    setStatusMeta(meta);
  }, []);

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
        <BondedNymNodeActions
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

  useEffect(() => {
    setStatusMeta(null);
  }, [identityKey]);

  const showWalletName = !isUnsetNodeName(name);
  const apiMoniker = statusMeta?.displayMoniker?.trim();
  const showApiMoniker = !showWalletName && Boolean(apiMoniker);
  const locationChip = statusMeta?.locationLabel;

  return (
    <Stack gap={2}>
      <NymCard
        borderless
        title={
          <Stack gap={2}>
            <Box display="flex" alignItems="center" gap={2} flexWrap="wrap">
              <Typography variant="h5" fontWeight={600}>
                Nym node
              </Typography>
              {locationChip ? (
                <Typography variant="body2" color="text.secondary" sx={{ fontWeight: 500 }}>
                  {locationChip}
                </Typography>
              ) : null}
            </Box>
            {showWalletName ? (
              <Typography fontWeight="regular" variant="h6" width="fit-content">
                {name}
              </Typography>
            ) : null}
            {showApiMoniker ? (
              <Typography fontWeight="regular" variant="h6" width="fit-content" color="text.primary">
                {apiMoniker}
              </Typography>
            ) : null}
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
                  >
                    Node Settings
                  </Button>
                </Box>
              </Tooltip>
            </Stack>

            {nextEpoch instanceof Error ? null : (
              <Typography fontSize={14} marginRight={1}>
                Next epoch starts at <b>{nextEpoch}</b>
              </Typography>
            )}
          </Box>
        }
      >
        <Stack spacing={3} sx={{ width: '100%', minWidth: 0 }}>
          <Box sx={{ width: '100%', minWidth: 0 }}>
            <NodeTable headers={headers} cells={cells} />
            {network ? (
              <Typography sx={{ mt: 2, fontSize: 'small' }}>
                Check more stats of your node on the{' '}
                <Link href={`${urls(network).networkExplorer}/nodes/${nodeId}`} target="_blank">
                  explorer
                </Link>
              </Typography>
            ) : null}
          </Box>
          <NodeOperatorInsights
            network={network}
            identityKey={identityKey}
            walletUptime={uptime}
            onStatusLoaded={handleStatusLoaded}
          />
        </Stack>
      </NymCard>
    </Stack>
  );
};
