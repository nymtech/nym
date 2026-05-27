import React from 'react';
import { Alert, Box, Skeleton, Stack, Typography } from '@mui/material';
import { alpha } from '@mui/material/styles';
import { Balance, decimalToFloatApproximation } from '@nymproject/types';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';
import { ClientAddress } from '@nymproject/react/client-address/ClientAddress';
import { Network } from 'src/types';
import { NymCard } from '../../components';
import { urls } from '../../context/main';
import { useNymUsdPrice } from '../../hooks/useNymUsdPrice';

const usdFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

export const BalanceCard = ({
  userBalance,
  userBalanceError,
  network,
  clientAddress,
  isLoading,
}: {
  userBalance?: Balance;
  userBalanceError?: string;
  network?: Network;
  clientAddress?: string;
  isLoading?: boolean;
}) => {
  const { usdPerNym, loading: priceLoading } = useNymUsdPrice(network);

  const nymFloat =
    userBalance?.amount?.amount && userBalance.amount.amount.length > 0
      ? decimalToFloatApproximation(userBalance.amount.amount)
      : undefined;

  const usdApproxLabel =
    usdPerNym !== undefined && nymFloat !== undefined ? usdFormatter.format(nymFloat * usdPerNym) : undefined;

  const showUsdRow = Boolean(userBalance?.amount?.amount && userBalance.amount.amount.length > 0);

  let usdApproximationRow: React.ReactNode = null;
  if (showUsdRow) {
    if (priceLoading) {
      usdApproximationRow = <Skeleton width={140} height={22} sx={{ mt: 0.5 }} />;
    } else if (usdApproxLabel) {
      usdApproximationRow = (
        <Typography
          variant="body2"
          sx={{ color: 'nym.text.muted', fontWeight: 500, mt: 0.25 }}
          data-testid="balance-usd-approx"
        >
          {`≈ ${usdApproxLabel}`}
        </Typography>
      );
    }
  }

  return (
    <NymCard
      title="Balance"
      subheader="Your primary spendable NYM balance"
      dataTestid="check-balance"
      borderless
      Action={clientAddress && <ClientAddress address={clientAddress} withCopy showEntireAddress />}
      sx={{
        backgroundColor: 'background.paper',
      }}
    >
      <Stack spacing={2.5}>
        <Box>
          {userBalanceError && (
            <Alert severity="error" data-testid="error-refresh" sx={{ p: 2 }}>
              {userBalanceError}
            </Alert>
          )}
          {isLoading ? (
            <Skeleton width={160} height={42} />
          ) : (
            !userBalanceError && (
              <Stack spacing={1}>
                <Typography
                  variant="caption"
                  sx={{ color: 'nym.text.muted', textTransform: 'uppercase', letterSpacing: 1 }}
                >
                  Available now
                </Typography>
                <Typography
                  data-testid="refresh-success"
                  sx={{
                    color: 'text.primary',
                    textTransform: 'uppercase',
                    fontWeight: 700,
                    fontSize: { xs: 30, md: 40 },
                    lineHeight: 1,
                  }}
                  variant="h5"
                >
                  {userBalance?.printable_balance || '-'}
                </Typography>
                {usdApproximationRow}
              </Stack>
            )
          )}
        </Box>
        {network && clientAddress && (
          <Box
            sx={{
              display: 'flex',
              flexDirection: { xs: 'column', sm: 'row' },
              gap: { xs: 1.5, sm: 2 },
              py: 1.5,
              px: 2,
              borderRadius: 3,
              border: '1px solid',
              borderColor: 'divider',
              bgcolor: (t) => alpha(t.palette.text.primary, t.palette.mode === 'dark' ? 0.04 : 0.06),
            }}
          >
            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Typography
                variant="caption"
                sx={{ color: 'nym.text.muted', textTransform: 'uppercase', letterSpacing: 0.8 }}
              >
                Destination
              </Typography>
              <Typography sx={{ mt: 0.5, fontWeight: 600, fontSize: 14 }}>Main account</Typography>
            </Box>
            <Box
              sx={{
                display: { xs: 'none', sm: 'block' },
                width: '1px',
                alignSelf: 'stretch',
                bgcolor: 'divider',
                flexShrink: 0,
              }}
            />
            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Typography
                variant="caption"
                sx={{ color: 'nym.text.muted', textTransform: 'uppercase', letterSpacing: 0.8 }}
              >
                Activity
              </Typography>
              <Box sx={{ mt: 0.5 }}>
                <Link
                  href={`${urls(network).mixnetExplorer}account/${clientAddress}`}
                  target="_blank"
                  text="View latest transactions"
                  fontSize={14}
                />
              </Box>
            </Box>
          </Box>
        )}
      </Stack>
    </NymCard>
  );
};
