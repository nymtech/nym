import React from 'react';
import { Alert, Grid, Typography, Skeleton } from '@mui/material';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';
import { ClientAddress } from '@nymproject/react/client-address/ClientAddress';
import { Network } from 'src/types';
import { Balance } from '@nymproject/types';
import { NymCard } from '../../components';
import { urls } from '../../context/main';

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
}) => (
  <NymCard
    title="Balance"
    data-testid="check-balance"
    borderless
    Action={clientAddress && <ClientAddress address={clientAddress} withCopy showEntireAddress />}
  >
    <Grid container direction="column" spacing={2}>
      <Grid item>
        {userBalanceError && (
          <Alert severity="error" data-testid="error-refresh" sx={{ p: 2 }}>
            {userBalanceError}
          </Alert>
        )}
        {isLoading ? (
          <Skeleton width={160} height={42} />
        ) : (
          !userBalanceError && (
            <Typography
              data-testid="refresh-success"
              sx={{
                color: 'text.primary',
                textTransform: 'uppercase',
                fontWeight: '600',
                fontSize: 28,
              }}
              variant="h5"
            >
              {userBalance?.printable_balance || '—'}
            </Typography>
          )
        )}
      </Grid>
      {network && (
        <Grid item>
          <Link
            href={`${urls(network).mixnetExplorer}account/${clientAddress}`}
            target="_blank"
            text="Last transactions"
            fontSize={14}
          />
        </Grid>
      )}
    </Grid>
  </NymCard>
);
