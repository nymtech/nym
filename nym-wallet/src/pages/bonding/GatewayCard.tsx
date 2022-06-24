import React, { useMemo } from 'react';
import { IconButton } from '@mui/material';
import { MoreVert } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { NymCard } from '../../components';
import { BondedGateway } from '../../context';
import { Cell, Header, NodeTable } from './NodeTable';
import BondedNodeCard from './BondedNodeCard';
import { bondGateway } from '../../requests';

const headers: Header[] = [
  {
    header: 'IP',
    id: 'ip-header',
    sx: { pl: 0 },
  },
  {
    header: 'Bond',
    id: 'bond-header',
  },
  {
    id: 'menu-button',
    size: 'small',
    sx: { pr: 0 },
  },
];

export const GatewayCard = ({ gateway }: { gateway: BondedGateway }) => {
  const { ip, bond } = gateway;
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
    [gateway, theme],
  );
  return (
    <BondedNodeCard title="Valhalla gateway" identityKey={gateway.key}>
      <NodeTable headers={headers} cells={cells} />
    </BondedNodeCard>
  );
};
