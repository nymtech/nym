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
    header: 'Bond',
    id: 'bond',
  },

  {
    header: 'Routing score',
    id: 'routing-score',
    tooltipText: 'Routing score',
  },
  {
    header: 'Average score',
    id: 'average-score',
    tooltipText: 'Average score',
  },
  {
    header: 'IP',
    id: 'ip',
  },
  {
    id: 'menu-button',
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
  const { name, bond, ip, identityKey, routingScore } = gateway;
  const cells: Cell[] = [
    {
      cell: `${bond.amount} ${bond.denom}`,
      id: 'bond-cell',
      sx: { pl: 0 },
    },

    {
      cell: `${routingScore?.current || '- '}%`,
      id: 'routing-score-cell',
    },
    {
      cell: `${routingScore?.average || '- '}%`,
      id: 'average-score-cell',
    },
    {
      cell: ip,
      id: 'ip-cell',
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
        <Stack gap={3}>
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
          Check more stats of your gateway on the{' '}
          <Link href={`${urls(network).networkExplorer}/network-components/gateway/${identityKey}`} target="_blank">
            explorer
          </Link>
        </Typography>
      )}
    </NymCard>
  );
};
