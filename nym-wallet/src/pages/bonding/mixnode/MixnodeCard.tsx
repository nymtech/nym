import React, { useMemo, useState } from 'react';
import { Button, IconButton, Typography } from '@mui/material';
import { MoreVert } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { Link } from '@nymproject/react/link/Link';
import { BondedMixnode } from '../../../context';
import { Node as NodeIcon } from '../../../svg-icons/node';
import { NodeTable, BondedNodeCard, Cell, Header } from '../components';
import { NodeSettings } from './node-settings';

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

const MixnodeCard = ({ mixnode }: { mixnode: BondedMixnode }) => {
  const { stake, bond, stakeSaturation, profitMargin, nodeRewards, operatorRewards, delegators } = mixnode;
  const [showNodeSettings, setShowNodeSettings] = useState<boolean>(false);
  const theme = useTheme();
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
        color: stakeSaturation > 100 ? theme.palette.nym.nymWallet.selectionChance.underModerate : undefined,
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
    [mixnode, theme],
  );
  return (
    <BondedNodeCard
      title="Monster node"
      identityKey={mixnode.key}
      status={mixnode.status}
      action={
        <Button
          variant="text"
          color="secondary"
          onClick={() => setShowNodeSettings(true)}
          sx={{
            fontWeight: 500,
            '& .MuiSvgIcon-root': {
              fontSize: 14,
            },
          }}
          startIcon={<NodeIcon />}
        >
          Node settings
        </Button>
      }
    >
      <NodeTable headers={headers} cells={cells} />
      <Typography sx={{ mt: 2 }}>
        Check more stats of your node on the{' '}
        <Link href="url" target="_blank">
          explorer
        </Link>
      </Typography>
      <NodeSettings mixnode={mixnode} open={showNodeSettings} />
    </BondedNodeCard>
  );
};

export default MixnodeCard;
