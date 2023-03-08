import React, { useContext, useEffect } from 'react';
import { Alert, Grid, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { NymCard, ClientAddress } from '../../components';
import { AppContext, urls } from '../../context/main';
import { Network } from 'src/types';
import { Balance } from '@nymproject/types';

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
}) => {
  return (
    <NymCard
      title="Balance"
      data-testid="check-balance"
      borderless
      Action={<ClientAddress withCopy showEntireAddress />}
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
};
