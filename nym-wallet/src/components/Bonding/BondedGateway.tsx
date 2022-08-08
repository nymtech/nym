import React from 'react';
import { Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { TBondedGateway, urls } from 'src/context';
import { NymCard } from 'src/components';
import { Network } from 'src/types';
import { IdentityKey } from 'src/components/IdentityKey';
import { Cell, Header, NodeTable } from './NodeTable';
import { BondedGatewayActions, TBondedGatwayActions } from './BondedGatewayAction';

const headers: Header[] = [
  {
    header: 'IP',
    id: 'ip',
    sx: { pl: 0 },
  },
  {
    header: 'Bond',
    id: 'bond',
  },
  {
    id: 'menu-button',
    sx: { width: 34, maxWidth: 34 },
  },
];

export const BondedGateway = ({
  gateway,
  network,
  onActionSelect,
}: {
  gateway: TBondedGateway;
  network?: Network;
  onActionSelect: (action: TBondedGatwayActions) => void;
}) => {
  const { name, bond, ip, identityKey } = gateway;
  const cells: Cell[] = [
    {
      cell: ip,
      id: 'stake-saturation-cell',
    },
    {
      cell: `${bond.amount} ${bond.denom}`,
      id: 'stake-cell',
      sx: { pl: 0 },
    },

    {
      cell: <BondedGatewayActions onActionSelect={onActionSelect} />,
      id: 'actions-cell',
      align: 'right',
    },
  ];

  return (
    <NymCard
      borderless
      title={
        <Stack gap={2}>
          <Typography variant="h5" fontWeight={600}>
            Gateway
          </Typography>

          {name && (
            <Typography fontWeight="regular" variant="h6">
              {name}
            </Typography>
          )}
          <IdentityKey identityKey={identityKey} />
        </Stack>
      }
    >
      <NodeTable headers={headers} cells={cells} />
      {network && (
        <Typography sx={{ mt: 2, fontSize: 'small' }}>
          Check more stats of your node on the{' '}
          <Link href={`${urls(network).networkExplorer}/network-components/gateways`} target="_blank">
            explorer
          </Link>
        </Typography>
      )}
    </NymCard>
  );
};
