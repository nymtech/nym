import React, { useMemo } from 'react';
import { IconButton, Typography } from '@mui/material';
import { MoreVert } from '@mui/icons-material';
import { Link } from '@nymproject/react/link/Link';
import { NymCard } from '../../components';
import { BondedMixnode } from '../../context';
import { Cell, Header, NodeTable } from './NodeTable';

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
    tooltip: 'TODO',
  },
  {
    header: 'PM',
    id: 'profit-margin',
    tooltip:
      'The percentage of the node rewards that you as the node operator will take before the rest of the reward is shared between you and the delegators.',
  },
  {
    header: 'Node rewards',
    id: 'node-rewards',
    tooltip: 'This is the total rewards for this node in this epoch including delegates and the operators share.',
  },
  {
    header: 'Operator rewards',
    id: 'operator-rewards',
    tooltip:
      'This is your (operator) new rewards including the PM and cost. You can compound your rewards manually every epoch or unbond your node to redeem them.',
  },
  {
    header: 'No. delegators',
    id: 'delegators',
  },
  {
    id: 'menu-button',
    size: 'small',
    sx: { pr: 0 },
  },
];

export const MixnodeCard = ({ mixnode }: { mixnode: BondedMixnode }) => {
  const { stake, bond, stakeSaturation, profitMargin, nodeRewards, operatorRewards, delegators } = mixnode;
  const cells: Cell[] = useMemo(
    () => [
      {
        cell: `${stake.amount} ${stake.denom}`,
        id: 'stake-cell',
        sx: { pl: 0 },
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
        cell: `${nodeRewards.amount} ${nodeRewards.denom}`,
        id: 'node-rewards-cell',
      },
      {
        cell: `${operatorRewards.amount} ${operatorRewards.denom}`,
        id: 'operator-rewards-cell',
      },
      {
        cell: delegators,
        id: 'delegators-cell',
      },
      {
        cell: (
          <IconButton sx={{ fontSize: '1rem', padding: 0 }}>
            <MoreVert fontSize="inherit" sx={{ color: 'text.primary' }} />
          </IconButton>
        ),
        id: 'menu-button-cell',
        align: 'center',
        size: 'small',
        sx: { pr: 0 },
      },
    ],
    [mixnode],
  );
  return (
    <NymCard title="Monster node">
      <NodeTable headers={headers} cells={cells} />
      <Typography sx={{ mt: 2 }}>
        Check more stats of your node on the{' '}
        <Link href="url" target="_blank">
          explorer
        </Link>
      </Typography>
    </NymCard>
  );
};
