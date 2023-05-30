import React from 'react';
import { PageLayout } from 'src/layouts/PageLayout';
import { Stack } from '@mui/material';
import { Add, ArrowDownward } from '@mui/icons-material';
import { AccountList, Button } from 'src/components';
import { ViewSeedPhrase } from 'src/components/accounts/ViewSeedPhrase';
import { useAppContext } from 'src/context';

export const Accounts = () => {
  const { showSeedForAccount, setShowSeedForAccount } = useAppContext();

  return (
    <PageLayout>
      {showSeedForAccount && (
        <ViewSeedPhrase accountName={showSeedForAccount} onDone={() => setShowSeedForAccount(undefined)} />
      )}
      <AccountList />
      <Stack gap={1} alignItems="start" sx={{ mt: 2 }}>
        <Button startIcon={<Add />}>Add account</Button>
        <Button startIcon={<ArrowDownward />}>Import account</Button>
      </Stack>
    </PageLayout>
  );
};
