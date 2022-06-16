import React, { useContext, useEffect } from 'react';
import { Alert, Button, Grid, Link, Typography } from '@mui/material';
import { OpenInNew } from '@mui/icons-material';
import { NymCard, ClientAddress } from '../../components';
import { AppContext, urls } from '../../context/main';

export const BalanceCard = () => {
  const { userBalance, clientDetails, network } = useContext(AppContext);

  useEffect(() => {
    userBalance.fetchBalance();
  }, []);

  return (
    <NymCard title="Balance" data-testid="check-balance" Action={<ClientAddress withCopy showEntireAddress />}>
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
              sx={{ color: 'nym.background.dark', textTransform: 'uppercase' }}
              variant="h5"
              fontWeight="700"
            >
              {userBalance.balance?.printable_balance}
            </Typography>
          )}
        </Grid>
        {network && (
          <Grid item>
            <Link href={`${urls(network).blockExplorer}/account/${clientDetails?.client_address}`} target="_blank">
              <Button endIcon={<OpenInNew />}>Last transactions</Button>
            </Link>
          </Grid>
        )}
      </Grid>
    </NymCard>
  );
};
