import React, { useEffect } from 'react';
import { Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { TBondedGateway, urls } from 'src/context';
import { NymCard } from 'src/components';
import { Network } from 'src/types';
import { IdentityKey } from 'src/components/IdentityKey';
import { getGatewayReport } from 'src/requests';
import { Cell, Header, NodeTable } from './NodeTable';
import { BondedGatewayActions, TBondedGatwayActions } from './BondedGatewayAction';
import { Console } from 'src/utils/console';

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
  const { name, bond, ip, identityKey } = gateway;
  const cells: Cell[] = [
    {
      cell: `${bond.amount} ${bond.denom}`,
      id: 'bond-cell',
      sx: { pl: 0 },
    },

    {
      cell: '100%',
      id: 'routing-score-cell',
    },
    {
      cell: '90%',
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

  const getGatewayReportDetails = async () => {
    try {
      const report = await getGatewayReport(gateway.identityKey);
      Console.log(report);
    } catch (e) {
      Console.error(e);
    }
  };

  useEffect(() => {
    getGatewayReportDetails();
  }, []);

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
          Check more stats of your node on the{' '}
          <Link href={`${urls(network).networkExplorer}/network-components/gateways`} target="_blank">
            explorer
          </Link>
        </Typography>
      )}
    </NymCard>
  );
};
