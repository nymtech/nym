import React, { useMemo } from 'react';
import { IconButton } from '@mui/material';
import { MoreVert } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { BondedGateway } from '../../../context';
import { NodeTable, BondedNodeCard, Cell, Header } from '../components';

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

const GatewayCard = ({ gateway }: { gateway: BondedGateway }) => {
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

export default GatewayCard;
