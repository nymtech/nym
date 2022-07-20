import React, { useEffect, useState } from 'react';
import { Stack, TextField, Typography } from '@mui/material';
import { SxProps } from '@mui/system';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, DecCoin } from '@nymproject/types';
import { validateAmount } from 'src/utils';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';

export const SendInputModal = ({
  fromAddress,
  toAddress,
  amount,
  balance,
  denom,
  error,
  onNext,
  onClose,
  onAmountChange,
  onAddressChange,
  sx,
  backdropProps,
}: {
  fromAddress?: string;
  toAddress: string;
  amount?: DecCoin;
  balance?: string;
  denom?: CurrencyDenom;
  error?: string;
  onNext: () => void;
  onClose: () => void;
  onAmountChange: (value: DecCoin) => void;
  onAddressChange: (value: string) => void;
  sx?: SxProps;
  backdropProps?: object;
}) => {
  const [isValid, setIsValid] = useState(false);

  const validate = async (value: DecCoin) => {
    const isValidAmount = await validateAmount(value.amount, '0');
    setIsValid(isValidAmount);
  };

  useEffect(() => {
    if (amount) validate(amount);
  }, []);

  return (
    <SimpleModal
      header="Send"
      open
      onClose={onClose}
      okLabel="Next"
      onOk={async () => onNext()}
      okDisabled={!isValid}
      sx={sx}
      backdropProps={backdropProps}
    >
      <Stack gap={2} sx={{ mt: 2 }}>
        <TextField
          placeholder="Recipient address"
          fullWidth
          onChange={(e) => onAddressChange(e.target.value)}
          value={toAddress}
        />
        <CurrencyFormField
          placeholder="Amount"
          fullWidth
          onChanged={(value) => {
            onAmountChange(value);
            validate(value);
          }}
          initialValue={amount?.amount}
          denom={denom}
        />
        <Typography fontSize="smaller" sx={{ color: 'error.main' }}>
          {error}
        </Typography>
      </Stack>
      <Stack gap={0.5} sx={{ mt: 2 }}>
        <ModalListItem label="Account balance" value={balance} divider strong />
        <ModalListItem label="Your address" value={fromAddress} divider />
        <Typography fontSize="smaller" sx={{ color: 'text.primary' }}>
          Est. fee for this transaction will be show on the next page
        </Typography>
      </Stack>
    </SimpleModal>
  );
};
