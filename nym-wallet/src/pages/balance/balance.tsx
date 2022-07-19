import React, { useContext, useEffect } from 'react';
import { Alert, Grid, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { NymCard, ClientAddress } from '../../components';
import { AppContext, urls } from '../../context/main';

export const BalanceCard = () => {
  const { userBalance, clientDetails, network } = useContext(AppContext);

  useEffect(() => {
    userBalance.fetchBalance();
  }, []);

  return (
    <NymCard
      title="Balance"
      titleSx={{ fontSize: 20 }}
      data-testid="check-balance"
      borderless
      Action={<ClientAddress withCopy showEntireAddress />}
    >
      <Grid container direction="column" spacing={2}>
        <Grid item>
          {userBalance.error && (
            <Alert severity="error" data-testid="error-refresh" sx={{ p: 2 }}>
              {userBalance.error}
            </Alert>
          )}
          {!userBalance.error && (
            <Typography
              data-testid="refresh-success"
              sx={{
                color: 'text.primary',
                textTransform: 'uppercase',
                fontWeight: (theme) => (theme.palette.mode === 'light' ? '600' : '400'),
                fontSize: 28,
              }}
              variant="h5"
            >
              {userBalance.balance?.printable_balance}
            </Typography>
          )}
        </Grid>
        {network && (
          <Grid item>
            <Link
              href={`${urls(network).blockExplorer}/account/${clientDetails?.client_address}`}
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
