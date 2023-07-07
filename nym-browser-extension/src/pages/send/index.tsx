import React, { useState } from 'react';
import { Box, Divider, Stack, Typography } from '@mui/material';
import { WalletAddressFormField } from '@nymproject/react/account/WalletAddressFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { DecCoin } from '@nymproject/types';
import { Address, Button } from 'src/components';
import { PageLayout } from 'src/layouts/PageLayout';
import { SendProvider, useAppContext, useSendContext } from 'src/context';
import { SendConfirmationPage } from './Confirmation';

const SendPage = ({ onConfirm }: { onConfirm: () => void }) => {
  const [isValidAddress, setIsValidAddress] = useState(false);
  const [isValidAmount, setIsValidAmount] = useState(false);

  const { address, amount, handleChangeAddress, handleChangeAmount, handleGetFee } = useSendContext();
  const { balance } = useAppContext();

  const handleNext = async () => {
    if (address && amount) {
      await handleGetFee(address, amount.amount);
      onConfirm();
    }
  };

  return (
    <PageLayout>
      <Stack gap={4} height="100%">
        <Address />
        <WalletAddressFormField
          showTickOnValid
          label="Recipient address"
          required
          onChanged={(_address: string) => handleChangeAddress(_address)}
          onValidate={setIsValidAddress}
          initialValue={address}
        />
        <CurrencyFormField
          label="Amount"
          initialValue={amount?.amount}
          required
          onChanged={(_amount: DecCoin) => handleChangeAmount(_amount)}
          onValidate={(_: any, isValid: boolean) => setIsValidAmount(isValid)}
        />
        <Box>
          <Stack direction="row" justifyContent="space-between">
            <Typography fontWeight={600}>Account balance</Typography>
            <Typography fontWeight={600}>{balance} NYM</Typography>
          </Stack>
          <Divider sx={{ my: 2 }} />
          <Typography variant="body2" sx={{ color: 'grey.600' }}>
            Est. fee for this transaction will be calculated on the next page
          </Typography>
        </Box>
      </Stack>
      <Button
        variant="contained"
        size="large"
        fullWidth
        disabled={!(isValidAddress && isValidAmount)}
        onClick={handleNext}
      >
        Next
      </Button>
    </PageLayout>
  );
};

export const Send = () => {
  const [showConfirmation, setShowConfirmation] = useState(false);

  return (
    <SendProvider>
      {showConfirmation ? (
        <SendConfirmationPage onCancel={() => setShowConfirmation(false)} />
      ) : (
        <SendPage onConfirm={() => setShowConfirmation(true)} />
      )}
    </SendProvider>
  );
};
