import React, { useEffect, useState } from 'react';
import { Stack, TextField, Typography, SxProps, FormControlLabel, Checkbox } from '@mui/material';
import Big from 'big.js';
import { CurrencyFormField } from '@nymproject/react';
import { CurrencyDenom, DecCoin, isValidRawCoin } from '@nymproject/types';
import { validateAmount } from 'src/utils';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';

const maxUserFees = '10.0';
const minUserFees = '0.000001'; // aka 1 unym

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
  userFees,
  memo,
  onUserFeesChange,
  onMemoChange,
  showMore,
  setShowMore,
}: {
  fromAddress?: string;
  toAddress: string;
  amount?: DecCoin;
  balance?: string;
  denom?: CurrencyDenom;
  error?: string;
  showMore?: boolean;
  setShowMore: (show: boolean) => void;
  onNext: () => void;
  onClose: () => void;
  onAmountChange: (value: DecCoin) => void;
  onAddressChange: (value: string) => void;
  sx?: SxProps;
  backdropProps?: object;
  userFees?: DecCoin;
  memo?: string;
  onUserFeesChange: (value: DecCoin) => void;
  onMemoChange: (value: string) => void;
}) => {
  const [isValid, setIsValid] = useState(false);
  const [memoIsValid, setMemoIsValid] = useState(true);
  const [feeAmountIsValid, setFeeAmountIsValid] = useState(true);

  const validate = async (value: DecCoin) => {
    const isValidAmount = await validateAmount(value.amount, '0');
    setIsValid(isValidAmount);
  };

  const validateUserFees = (fees: DecCoin) => {
    if (!isValidRawCoin(fees.amount) || !Number(fees.amount)) {
      setFeeAmountIsValid(false);
      return;
    }
    const f = Big(fees.amount);
    if (f.gt(maxUserFees) || f.lt(minUserFees)) {
      setFeeAmountIsValid(false);
      return;
    }
    setFeeAmountIsValid(true);
  };

  useEffect(() => {
    if (amount) validate(amount);
  }, []);

  useEffect(() => {
    if (memo && !/^(\w|\s)+$/.test(memo)) {
      setMemoIsValid(false);
      return;
    }
    setMemoIsValid(true);
  }, [memo]);

  useEffect(() => {
    if (userFees) {
      validateUserFees(userFees);
    } else {
      setFeeAmountIsValid(true);
    }
  }, [userFees]);

  return (
    <SimpleModal
      header="Send"
      open
      onClose={onClose}
      okLabel="Next"
      onOk={async () => onNext()}
      okDisabled={!isValid || !memoIsValid || !feeAmountIsValid}
      sx={sx}
      backdropProps={backdropProps}
    >
      <Stack gap={3}>
        <ModalListItem label="Your address" value={fromAddress} fontWeight="light" />
        <TextField
          label="Recipient address"
          fullWidth
          onChange={(e) => onAddressChange(e.target.value)}
          value={toAddress}
          InputLabelProps={{ shrink: true }}
        />
        <CurrencyFormField
          label="Amount"
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
      <Stack gap={0.5} sx={{ mt: 1 }}>
        <ModalListItem label="Account balance" value={balance?.toUpperCase()} divider fontWeight={600} />
        <Typography fontSize="smaller" sx={{ color: 'text.primary' }}>
          Est. fee for this transaction will be shown on the next page
        </Typography>
      </Stack>
      <FormControlLabel
        control={<Checkbox onChange={() => setShowMore(!showMore)} checked={showMore} />}
        label="More options"
        sx={{ mt: 2 }}
      />
      {showMore && (
        <Stack direction="column" gap={3} mt={2} mb={3}>
          <CurrencyFormField
            label="Fee"
            onChanged={(v) => onUserFeesChange(v)}
            initialValue={userFees?.amount}
            fullWidth
          />
          <TextField
            name="memo"
            label="Memo"
            onChange={(e) => onMemoChange(e.target.value)}
            value={memo}
            error={!memoIsValid}
            helperText={
              !memoIsValid
                ? ' The text is invalid, only alphanumeric characters and white spaces are allowed'
                : undefined
            }
            InputLabelProps={{ shrink: true }}
            fullWidth
          />
        </Stack>
      )}
    </SimpleModal>
  );
};
