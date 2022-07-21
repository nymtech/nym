import { useState } from 'react';
import { Stack, Typography } from '@mui/material';
import { NymCard } from 'src/components';
import { IdentityKey } from 'src/components/IdentityKey';
import { BondedGateway } from '../../../context';
import { Cell, Header, NodeMenu, NodeTable } from '../components';
import Unbond from '../unbond';
import { GatewayFlow } from './types';

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

  const cells: Cell[] = [
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
      cell: <NodeMenu onFlowChange={(newFlow) => setFlow(newFlow as GatewayFlow)} />,
      id: 'menu-button-cell',
      align: 'center',
    },
  ];

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
