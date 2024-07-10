import { Box, Button, Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react';
import { TBondedGateway, urls } from '@src/context';
import { NymCard } from '@src/components';
import { Network } from '@src/types';
import { IdentityKey } from '@src/components/IdentityKey';
import { useNavigate } from 'react-router-dom';
import { Node as NodeIcon } from '../../svg-icons/node';
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
    tooltipText:
      "Gateway's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test",
  },
  {
    header: 'Average score',
    id: 'average-score',
    tooltipText: "Gateway's average routing score in the last 24 hours",
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
  const navigate = useNavigate();
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
      Action={
        <Box>
          <Button
            variant="text"
            color="secondary"
            onClick={() => navigate('/bonding/node-settings')}
            startIcon={<NodeIcon />}
          >
            Gateway Settings
          </Button>
        </Box>
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
