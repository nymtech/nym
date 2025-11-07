import React from 'react';
import { Box, Typography, Grid, Link, Card, CardContent, Stack } from '@mui/material';
import BitfinexIcon from 'src/svg-icons/bitfinex.svg';
import KrakenIcon from 'src/svg-icons/kraken.svg';
import BybitIcon from 'src/svg-icons/bybit.svg';
import GateIcon from 'src/svg-icons/gate22.svg';
import HTXIcon from 'src/svg-icons/htx.svg';
import { NymCard } from '..';

const ExchangeCard = ({
  name,
  tokenType,
  url,
  IconComponent,
}: {
  name: string;
  tokenType: string;
  url: string;
  IconComponent: React.FunctionComponent<React.SVGProps<SVGSVGElement>>;
}) => (
  <Card
    variant="outlined"
    sx={{
      height: '100%',
      transition: 'all 0.2s ease-in-out',
      '&:hover': {
        transform: 'translateY(-2px)',
        boxShadow: 2,
      },
    }}
  >
    <CardContent sx={{ p: 3 }}>
      <Stack direction="row" spacing={2} alignItems="center">
        <Box
          sx={{
            width: 40,
            height: 40,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 1,
            bgcolor: 'background.paper',
          }}
        >
          <IconComponent width={24} height={24} />
        </Box>
        <Stack spacing={1} sx={{ flex: 1 }}>
          <Typography variant="h6" fontWeight={600}>
            {name}
          </Typography>
          <Typography variant="body2" sx={{ color: 'text.secondary' }}>
            {tokenType}
          </Typography>
          <Link
            href={url}
            target="_blank"
            variant="body2"
            data-testid="link-get-nym"
            sx={{
              textDecoration: 'underline',
              fontWeight: 500,
              '&:hover': {
                textDecoration: 'none',
              },
            }}
          >
            GET NYM
          </Link>
        </Stack>
      </Stack>
    </CardContent>
  </Card>
);

export const Tutorial = () => {
  const exchanges = [
    {
      name: 'Bitfinex',
      tokenType: 'Native NYM, ERC-20',
      url: 'https://www.bitfinex.com/',
      IconComponent: BitfinexIcon,
    },
    {
      name: 'Kraken',
      tokenType: 'Native NYM',
      url: 'https://www.kraken.com/',
      IconComponent: KrakenIcon,
    },
    {
      name: 'Bybit',
      tokenType: 'ERC-20',
      url: 'https://www.bybit.com/en/',
      IconComponent: BybitIcon,
    },
    {
      name: 'Gate.io',
      tokenType: 'ERC-20',
      url: 'https://www.gate.io/',
      IconComponent: GateIcon,
    },
    {
      name: 'HTX',
      tokenType: 'ERC-20',
      url: 'https://www.htx.com/',
      IconComponent: HTXIcon,
    },
  ];

  return (
    <NymCard borderless title="Where you can get NYM tokens" sx={{ mt: 4 }}>
      <Typography mb={3} fontSize={14} sx={{ color: 'text.secondary' }}>
        You can get NYM tokens from these exchanges
      </Typography>

      <Grid container spacing={3}>
        {exchanges.map((exchange) => (
          <Grid item xs={12} md={6} lg={4} key={exchange.name}>
            <ExchangeCard {...exchange} />
          </Grid>
        ))}
      </Grid>
    </NymCard>
  );
};
