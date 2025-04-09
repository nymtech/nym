import React, { useEffect, useState } from 'react';
import { Stack, Typography, SxProps, FormControlLabel, Checkbox, Alert } from '@mui/material';
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

  if (address.length !== 40) return false;

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
  
  // Calculate noAccount at the component root level instead of using useEffect
  const noAccount = !balance || balance === '0' || parseFloat(balance) === 0;

  const validateSendAmount = async (value: DecCoin) => {
    let newValidatedValue = true;
    let errorAmountMessage;

    if (noAccount) {
      newValidatedValue = false;
      errorAmountMessage = 'You need to acquire NYMs before sending. Try using the Buy section in the app.';
      setIsValid(newValidatedValue);
      setErrorAmount(errorAmountMessage);
      return newValidatedValue;
    }

    if (!value.amount) {
      newValidatedValue = false;
    } else {
      // Skip validation for partial decimal inputs during typing
      if (value.amount === '.' || value.amount.endsWith('.')) {
        newValidatedValue = false;
        setIsValid(newValidatedValue);
        setErrorAmount(undefined);
        return newValidatedValue;
      }

      if (!(await validateAmount(value.amount, '0'))) {
        newValidatedValue = false;
        errorAmountMessage = 'Please enter a valid amount';
      } else if (Number(value.amount) < Number(MIN_AMOUNT_TO_SEND)) {
        newValidatedValue = false;
        errorAmountMessage = `Min. send amount: ${MIN_AMOUNT_TO_SEND} ${denom?.toUpperCase()}`;
      } else if (balance && value.amount) {
        try {
          const amountBig = new Big(value.amount);
          const balanceBig = new Big(balance);

          if (amountBig.gt(balanceBig)) {
            newValidatedValue = false;
            errorAmountMessage = `Make sure you have sufficient funds. Available: ${balance} ${denom?.toUpperCase()}`;
          }
        } catch (err) {
          if (!/^\d*\.?\d*$/.test(value.amount)) {
            newValidatedValue = false;
            errorAmountMessage = 'Invalid number format';
          }
        }
      }
    }

    setIsValid(newValidatedValue);
    setErrorAmount(errorAmountMessage);
    return newValidatedValue;
  };

  const validateUserFees = (fees: DecCoin) => {
    let feeValid = true;
    let errorFeeMessage;

    if (noAccount) {
      feeValid = false;
      errorFeeMessage = 'You need to acquire NYMs before setting fees.';
      setFeeAmountIsValid(feeValid);
      setErrorFee(errorFeeMessage);
      return feeValid;
    }

    // Skip validation for partial decimal inputs during typing
    if (fees.amount === '.' || fees.amount.endsWith('.')) {
      setFeeAmountIsValid(false);
      setErrorFee(undefined);
      return false;
    }

    if (!isValidRawCoin(fees.amount) || !Number(fees.amount)) {
      feeValid = false;
      errorFeeMessage = 'Please enter a valid fee amount';
    } else {
      try {
        const f = Big(fees.amount);
        if (f.gt(maxUserFees)) {
          feeValid = false;
          errorFeeMessage = `Max fee: ${maxUserFees} ${denom?.toUpperCase()}`;
        } else if (f.lt(minUserFees)) {
          feeValid = false;
          errorFeeMessage = `Min. fee: ${minUserFees} ${denom?.toUpperCase()}`;
        }
      } catch (err) {
        if (!/^\d*\.?\d*$/.test(fees.amount)) {
          feeValid = false;
          errorFeeMessage = 'Invalid fee format';
        }
      }
    }

    setFeeAmountIsValid(feeValid);
    setErrorFee(errorFeeMessage);
    return feeValid;
  };

  useEffect(() => {
    if (amount) validateSendAmount(amount);
  }, [amount, balance, noAccount]);

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
  }, [userFees, noAccount]);

  return (
    <SimpleModal
      header="Send"
      open
      onClose={onClose}
      okLabel="Next"
      onOk={async () => onNext()}
      okDisabled={!isValid || !memoIsValid || !feeAmountIsValid || !addressIsValid || noAccount}
      sx={sx}
      backdropProps={backdropProps}
    >
      {noAccount && (
        <Alert severity="warning" sx={{ mb: 3 }}>
          To start staking, sending or operating the on the NYM network, you first need to get native NYM tokens.
        </Alert>
      )}

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
              ? 'Invalid NYM address. Must start with n1 and be exactly 40 characters long.'
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
        <ModalListItem label="Account balance" value={balance ? balance.toUpperCase() : '0'} divider fontWeight={600} />
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