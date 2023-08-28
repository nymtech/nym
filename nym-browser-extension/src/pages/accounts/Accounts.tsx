import React, { useEffect } from 'react';
import { PageLayout } from 'src/layouts/PageLayout';
import { Stack } from '@mui/material';
import { Add, ArrowDownward } from '@mui/icons-material';
import { AccountList, Button } from 'src/components';
import { ViewSeedPhrase } from 'src/components/accounts/ViewSeedPhrase';
import { useAppContext, useRegisterContext } from 'src/context';
import { useLocation, useNavigate } from 'react-router-dom';

export const Accounts = () => {
  const { showSeedForAccount, setShowSeedForAccount } = useAppContext();
  const { resetState } = useRegisterContext();

  useEffect(() => {
    resetState();
  }, []);

  const location = useLocation();
  const navigate = useNavigate();

  const handleAddAccount = () => navigate(`${location.pathname}/add-account`);

  const handleImportAccount = () => navigate(`${location.pathname}/import-account`);

  const onBack = () => navigate('/user/balance');

  return (
    <PageLayout onBack={onBack}>
      {showSeedForAccount && (
        <ViewSeedPhrase accountName={showSeedForAccount} onDone={() => setShowSeedForAccount(undefined)} />
      )}
      <AccountList />
      <Stack gap={1} alignItems="start" sx={{ mt: 2 }}>
        <Button startIcon={<Add />} onClick={handleAddAccount}>
          Add account
        </Button>
        <Button startIcon={<ArrowDownward />} onClick={handleImportAccount}>
          Import account
        </Button>
      </Stack>
    </PageLayout>
  );
};
