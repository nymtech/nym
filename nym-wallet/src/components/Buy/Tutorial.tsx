import React from 'react';
import { Box, Typography, Grid, Card, CardContent, Stack, Button } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { useSnackbar } from 'notistack';
import { safeOpenUrl } from 'src/utils/safeOpenUrl';
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
  onOpenExchange,
}: {
  name: string;
  tokenType: string;
  url: string;
  IconComponent: React.FunctionComponent<React.SVGProps<SVGSVGElement>>;
  onOpenExchange: (name: string, url: string) => void;
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
          <Button
            variant="text"
            data-testid="link-get-nym"
            onClick={() => onOpenExchange(name, url)}
            sx={{
              alignSelf: 'flex-start',
              p: 0,
              minWidth: 0,
              textTransform: 'none',
              textDecoration: 'underline',
              fontWeight: 500,
              fontSize: '0.875rem',
              '&:hover': {
                textDecoration: 'none',
                background: 'transparent',
              },
            }}
          >
            GET NYM
          </Button>
        </Stack>
      </Stack>
    </CardContent>
  </Card>
);

export const Tutorial = () => {
  const theme = useTheme();
  const { enqueueSnackbar } = useSnackbar();

  const openExchange = async (name: string, url: string) => {
    try {
      enqueueSnackbar(`Opening ${name} in your default browser - always verify the URL in the address bar.`, {
        variant: 'info',
      });
      await safeOpenUrl(url);
    } catch (e) {
      enqueueSnackbar('Could not open the link. Copy the URL from the exchange website instead.', {
        variant: 'error',
      });
    }
  };

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
    <NymCard
      borderless
      title="Where you can get NYM tokens"
      sx={{
        backgroundColor: 'background.paper',
        border: `1px solid ${theme.palette.divider}`,
        boxShadow: theme.palette.nym.nymWallet.shadows.light,
      }}
    >
      <Typography mb={3} fontSize={14} sx={{ color: 'text.secondary' }}>
        You can get NYM tokens from these exchanges
      </Typography>

      <Grid container spacing={3}>
        {exchanges.map((exchange) => (
          <Grid item xs={12} md={6} lg={4} key={exchange.name}>
            <ExchangeCard {...exchange} onOpenExchange={openExchange} />
          </Grid>
        ))}
      </Grid>
    </NymCard>
  );
};
