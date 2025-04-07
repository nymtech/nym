import React, { useEffect, useState } from 'react';
import { Stack, Typography, SxProps, FormControlLabel, Checkbox } from '@mui/material';
import Big from 'big.js';
import { CurrencyDenom, DecCoin, isValidRawCoin } from '@nymproject/types';
import { validateAmount } from 'src/utils';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';
import { TextFieldWithPaste, CurrencyFormFieldWithPaste } from '../Clipboard/ClipboardFormFields';

const maxUserFees = '10.0';
const minUserFees = '0.000001'; // aka 1 unym
const MIN_AMOUNT_TO_SEND = '0.000001'; // Adjust this as needed

// NYM address validation function
const validateNymAddress = (address: string): boolean => {
  if (!address) return false;

  if (!address.startsWith('n1')) return false;

  if (address.length < 40 || address.length > 42) return false;

  const validCharsRegex = /^[a-z0-9]+$/;
  return validCharsRegex.test(address);
};

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
  const [addressIsValid, setAddressIsValid] = useState(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();
  const [errorFee, setErrorFee] = useState<string | undefined>();

  const validateSendAmount = async (value: DecCoin) => {
    let newValidatedValue = true;
    let errorAmountMessage;

    if (!value.amount) {
      newValidatedValue = false;
    } else {
      // Validate amount format
      if (!(await validateAmount(value.amount, '0'))) {
        newValidatedValue = false;
        errorAmountMessage = 'Please enter a valid amount';
      }

      // Check minimum amount
      if (Number(value.amount) < Number(MIN_AMOUNT_TO_SEND)) {
        newValidatedValue = false;
        errorAmountMessage = `Min. send amount: ${MIN_AMOUNT_TO_SEND} ${denom?.toUpperCase()}`;
      }
    }

    setIsValid(newValidatedValue);
    setErrorAmount(errorAmountMessage);
    return newValidatedValue;
  };

  const validateUserFees = (fees: DecCoin) => {
    let isValid = true;
    let errorFeeMessage;

    if (!isValidRawCoin(fees.amount) || !Number(fees.amount)) {
      isValid = false;
      errorFeeMessage = 'Please enter a valid fee amount';
    } else {
      const f = Big(fees.amount);
      if (f.gt(maxUserFees)) {
        isValid = false;
        errorFeeMessage = `Max fee: ${maxUserFees} ${denom?.toUpperCase()}`;
      } else if (f.lt(minUserFees)) {
        isValid = false;
        errorFeeMessage = `Min. fee: ${minUserFees} ${denom?.toUpperCase()}`;
      }
    }

    setFeeAmountIsValid(isValid);
    setErrorFee(errorFeeMessage);
    return isValid;
  };

  useEffect(() => {
    if (amount) validateSendAmount(amount);
  }, [amount]);

  // Effect to validate address whenever it changes
  useEffect(() => {
    setAddressIsValid(validateNymAddress(toAddress));
  }, [toAddress]);

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
      okDisabled={!isValid || !memoIsValid || !feeAmountIsValid || !addressIsValid}
      sx={sx}
      backdropProps={backdropProps}
    >
      <Stack gap={3}>
        <ModalListItem label="Your address" value={fromAddress} fontWeight="light" />

        {/* Recipient address field with paste button */}
        <TextFieldWithPaste
          label="Recipient address"
          fullWidth
          onChange={(e) => onAddressChange(e.target.value)}
          value={toAddress}
          error={toAddress !== '' && !addressIsValid}
          helperText={
            toAddress !== '' && !addressIsValid
              ? 'Invalid NYM address. Must start with n1 and be 40-42 characters long.'
              : undefined
          }
          InputLabelProps={{ shrink: true }}
          onPasteValue={onAddressChange}
        />

        {/* Amount field with paste button */}
        <CurrencyFormFieldWithPaste
          label="Amount"
          fullWidth
          onChanged={(value) => {
            onAmountChange(value);
            validateSendAmount(value);
          }}
          initialValue={amount?.amount}
          denom={denom}
          validationError={errorAmount}
        />

        {/* Memo field with paste button */}
        <TextFieldWithPaste
          name="memo"
          label="Memo"
          onChange={(e) => onMemoChange(e.target.value)}
          value={memo}
          error={!memoIsValid}
          placeholder="Optional"
          helperText={
            !memoIsValid ? 'The text is invalid, only alphanumeric characters and white spaces are allowed' : undefined
          }
          InputLabelProps={{ shrink: true }}
          fullWidth
          onPasteValue={onMemoChange}
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
        <Stack mt={2} mb={3}>
          <CurrencyFormFieldWithPaste
            label="Fee"
            onChanged={(v) => {
              onUserFeesChange(v);
              validateUserFees(v);
            }}
            initialValue={userFees?.amount}
            fullWidth
            denom={denom}
            validationError={errorFee}
          />
        </Stack>
      )}
    </SimpleModal>
  );
};
