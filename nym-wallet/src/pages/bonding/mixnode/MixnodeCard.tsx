import { useMemo, useState } from 'react';
import { Button, Stack, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { Link } from '@nymproject/react/link/Link';
import { NymCard } from 'src/components';
import { IdentityKey } from 'src/components/IdentityKey';
import { NodeStatus } from 'src/components/NodeStatus';
import { BondedMixnode } from '../../../context';
import { Bond as BondIcon, Unbond as UnbondIcon } from '../../../svg-icons';
import { Node as NodeIcon } from '../../../svg-icons/node';
import { Cell, Header, NodeMenu, NodeTable } from '../components';
import Unbond from '../unbond';
import BondMore from './bond-more';
import CompoundRewards from './compound';
import NodeSettings from './node-settings';
import RedeemRewards from './redeem';
import { MixnodeFlow } from './types';

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
    tooltip: 'TODO', // TODO
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
    sx: { width: 34, maxWidth: 34 },
  },
];

const MixnodeCard = ({ mixnode }: { mixnode: BondedMixnode }) => {
  const { stake, bond, stakeSaturation, profitMargin, nodeRewards, operatorRewards, delegators } = mixnode;
  const [flow, setFlow] = useState<MixnodeFlow>(null);
  const [nodeMenuOpen, setNodeMenuOpen] = useState(false);
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
          <NodeMenu
            onFlowChange={(newFlow) => setFlow(newFlow as MixnodeFlow)}
            onOpen={(open) => setNodeMenuOpen(open)}
            items={[
              { label: 'Bond more', flow: 'bondMore', icon: <BondIcon fontSize="inherit" /> },
              { label: 'Unbond', flow: 'unbond', icon: <UnbondIcon fontSize="inherit" /> },
              {
                label: 'Compound rewards',
                flow: 'compound',
                icon: <Typography fontWeight={700}>C</Typography>,
                description: 'Add operator rewards to bond',
              },
              {
                label: 'Redeem rewards',
                flow: 'redeem',
                icon: <Typography fontWeight={700}>R</Typography>,
                description: 'Add your rewards to bonding pool',
              },
            ]}
          />
        ),
        id: 'menu-button-cell',
        align: 'center',
        size: 'small',
        sx: { backgroundColor: nodeMenuOpen ? '#FB6E4E0D' : undefined, px: 0 },
      },
    ],
    [mixnode, theme, nodeMenuOpen],
  );
  return (
    <NymCard
      title={
        <Stack gap={2}>
          <NodeStatus status={mixnode.status} />
          <Typography variant="h5">Monster node</Typography>
          <IdentityKey identityKey={mixnode.identityKey} />
        </Stack>
      }
      Action={
        <Button variant="text" color="secondary" onClick={() => setFlow('nodeSettings')} startIcon={<NodeIcon />}>
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
      <NodeSettings mixnode={mixnode} show={flow === 'nodeSettings'} onClose={() => setFlow(null)} />
      <BondMore mixnode={mixnode} show={flow === 'bondMore'} onClose={() => setFlow(null)} />
      <RedeemRewards mixnode={mixnode} show={flow === 'redeem'} onClose={() => setFlow(null)} />
      <Unbond node={mixnode} show={flow === 'unbond'} onClose={() => setFlow(null)} />
      <CompoundRewards mixnode={mixnode} show={flow === 'compound'} onClose={() => setFlow(null)} />
    </NymCard>
  );
};

export default MixnodeCard;
