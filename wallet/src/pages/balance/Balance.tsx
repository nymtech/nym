import { Alert, Grid, Typography } from '@mui/material';
import { Link } from '@nymproject/react';
import { ClientAddress } from '@nymproject/react';
import { Network } from '@src/types';
import { Balance } from '@nymproject/types';
import { NymCard } from '../../components';
import { urls } from '../../context/main';

export const BalanceCard = ({
  userBalance,
  userBalanceError,
  network,
  clientAddress,
}: {
  userBalance?: Balance;
  userBalanceError?: string;
  network?: Network;
  clientAddress?: string;
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
        {!userBalanceError && (
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
            {userBalance?.printable_balance}
          </Typography>
        )}
      </Grid>
      {network && (
        <Grid item>
          <Link
            href={`${urls(network).mixnetExplorer}/account/${clientAddress}`}
            target="_blank"
            text="Last transactions"
            fontSize={14}
          />
        </Grid>
      )}
    </Grid>
  </NymCard>
);
