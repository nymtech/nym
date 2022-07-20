import React, { useMemo, useState } from 'react';
import { useTheme } from '@mui/material/styles';
import EditIcon from '@mui/icons-material/Edit';
import { BondedGateway } from '../../../context';
import { NodeTable, Cell, Header, NodeMenu } from '../components';
import { GatewayFlow } from './types';
import Unbond from '../unbond';
import { NymCard } from 'src/components';
import { Stack, Typography } from '@mui/material';
import { IdentityKey } from 'src/components/IdentityKey';

const headers: Header[] = [
  {
    header: 'IP',
    id: 'ip-header',
    sx: { pl: 0, width: 100 },
  },
  {
    header: 'Bond',
    id: 'bond-header',
  },
  {
    id: 'menu-button',
    size: 'small',
    sx: { width: 34, maxWidth: 34 },
  },
];

const GatewayCard = ({ gateway }: { gateway: BondedGateway }) => {
  const { ip, bond } = gateway;
  const [flow, setFlow] = useState<GatewayFlow>(null);
  const [nodeMenuOpen, setNodeMenuOpen] = useState(false);
  const theme = useTheme();

  const cells: Cell[] = useMemo(
    () => [
      {
        cell: ip,
        id: 'ip-cell',
        sx: { pl: 0 },
      },
      {
        cell: `${bond.amount} ${bond.denom}`,
        id: 'bond-cell',
      },
      {
        cell: (
          <NodeMenu
            onFlowChange={(newFlow) => setFlow(newFlow as GatewayFlow)}
            onOpen={(open) => setNodeMenuOpen(open)}
            items={[{ label: 'Unbond', flow: 'unbond', icon: <EditIcon fontSize="inherit" /> }]}
          />
        ),
        id: 'menu-button-cell',
        align: 'center',
        size: 'small',
        sx: { backgroundColor: nodeMenuOpen ? '#FB6E4E0D' : undefined, px: 0 },
      },
    ],
    [gateway, theme, nodeMenuOpen],
  );
  return (
    <NymCard
      title={
        <Stack gap={2}>
          <Typography variant="h5">Valhalla gateway</Typography>
          <IdentityKey identityKey={gateway.identityKey} />
        </Stack>
      }
    >
      <NodeTable headers={headers} cells={cells} />
      <Unbond node={gateway} show={flow === 'unbond'} onClose={() => setFlow(null)} />
    </NymCard>
  );
};

export default GatewayCard;
