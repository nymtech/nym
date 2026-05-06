import React, { useCallback, useEffect, useState } from 'react';
import {
  Box,
  Button,
  Stack,
  Typography,
  SxProps,
  FormControlLabel,
  Checkbox,
  Alert,
  CircularProgress,
} from '@mui/material';
import { alpha, useTheme } from '@mui/material/styles';
import Big from 'big.js';
import { CurrencyDenom, DecCoin, isValidRawCoin } from '@nymproject/types';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { validateAmount } from 'src/utils';
import { validateNymAddress } from 'src/utils/validateNymAddress';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';
import { TextFieldWithPaste } from '../Clipboard/ClipboardFormFields';
import { CurrencyFormFieldWithPaste } from '../CurrencyFormFieldWithPaste';

const maxUserFees = '10.0';
const minUserFees = '0.000001'; // aka 1 unym
const MIN_AMOUNT_TO_SEND = '0.000001'; // Adjust this as needed

const recipientHelperText = (
  recipientTouched: boolean,
  toAddress: string,
  addressIsValid: boolean,
): string | undefined => {
  if (recipientTouched && !toAddress.trim()) {
    return 'Recipient address is required';
  }
  if (toAddress !== '' && !addressIsValid) {
    return 'Invalid NYM address. Must start with n1 and be exactly 40 characters long.';
  }
  return undefined;
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
  amountFieldKey,
  onMaxAmount,
  maxAmountLoading,
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
  /** Bump to remount the amount field after programmatic Max (CurrencyFormField is defaultValue-based). */
  amountFieldKey: number;
  onMaxAmount: () => void | Promise<void>;
  maxAmountLoading: boolean;
}) => {
  const [isValid, setIsValid] = useState(false);
  const [memoIsValid, setMemoIsValid] = useState(true);
  const [feeAmountIsValid, setFeeAmountIsValid] = useState(true);
  const [addressIsValid, setAddressIsValid] = useState(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();
  const [errorFee, setErrorFee] = useState<string | undefined>();
  const [recipientTouched, setRecipientTouched] = useState(false);
  /** Avoid "Amount is required" on modal open; show empty errors only after user edits the field. */
  const [amountTouched, setAmountTouched] = useState(false);
  /** Focus trap can fire a spurious blur on open; delay recipient blur validation slightly. */
  const [recipientBlurReady, setRecipientBlurReady] = useState(false);
  const theme = useTheme();

  // Calculate noAccount at the component root level instead of using useEffect
  const noAccount = !balance || balance === '0' || parseFloat(balance) === 0;

  const validateSendAmount = useCallback(
    async (value: DecCoin, assumeAmountInteracted = false) => {
      let newValidatedValue = true;
      let errorAmountMessage;
      const showEmptyFieldErrors = assumeAmountInteracted || amountTouched;

      if (noAccount) {
        newValidatedValue = false;
        errorAmountMessage = 'You need to acquire NYMs before sending. Try using the Buy section in the app.';
        setIsValid(newValidatedValue);
        setErrorAmount(errorAmountMessage);
        return newValidatedValue;
      }

      if (!value?.amount || String(value.amount).trim() === '') {
        newValidatedValue = false;
        errorAmountMessage = showEmptyFieldErrors ? 'Amount is required' : undefined;
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
              errorAmountMessage = `Amount exceeds your available balance. Available: ${balance} ${denom?.toUpperCase()}`;
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
    },
    [amountTouched, noAccount, balance, denom],
  );

  const validateUserFees = useCallback(
    (fees: DecCoin) => {
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
    },
    [noAccount, denom],
  );

  useEffect(() => {
    const id = window.setTimeout(() => setRecipientBlurReady(true), 400);
    return () => window.clearTimeout(id);
  }, []);

  useEffect(() => {
    const empty: DecCoin = { amount: '', denom: denom ?? 'nym' };
    validateSendAmount(amount ?? empty).catch(() => {
      /* validateSendAmount only updates state */
    });
  }, [amount, denom, validateSendAmount]);

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
  }, [userFees, validateUserFees]);

  return (
    <SimpleModal
      header="Send"
      subHeader="Enter the recipient address and amount to send NYM."
      headerCentered
      open
      onClose={onClose}
      okLabel="Next"
      onOk={async () => onNext()}
      okDisabled={!isValid || !memoIsValid || !feeAmountIsValid || !addressIsValid || noAccount}
      subHeaderStyles={{ mb: 0, px: 3, pt: 0.5 }}
      sx={{
        maxWidth: '480px',
        width: '100%',
        borderRadius: '20px',
        overflow: 'hidden',
        boxShadow: theme.palette.nym.nymWallet.shadows.strong,
        ...sx,
      }}
      backdropProps={backdropProps}
    >
      <Stack gap={3} sx={{ px: 3, pb: 3, pt: 0 }}>
        {noAccount && (
          <Alert severity="warning">
            To start staking, sending or operating on the NYM network, you first need to get native NYM tokens.
          </Alert>
        )}
        <Box sx={{ width: '100%' }}>
          <Typography
            variant="caption"
            sx={{
              mb: 1.5,
              color: 'text.secondary',
              fontWeight: 600,
              display: 'block',
              textAlign: 'center',
            }}
          >
            Your address
          </Typography>
          <Box
            sx={{
              p: 2,
              bgcolor: alpha(theme.palette.primary.main, 0.06),
              borderRadius: '12px',
              border: `1px solid ${alpha(theme.palette.primary.main, 0.18)}`,
            }}
          >
            {fromAddress ? (
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  justifyContent: 'space-between',
                  gap: 1.5,
                  width: '100%',
                }}
              >
                <Typography
                  component="span"
                  sx={{
                    flex: 1,
                    minWidth: 0,
                    fontSize: '0.9rem',
                    fontFamily: 'monospace',
                    letterSpacing: '0.5px',
                    wordBreak: 'break-all',
                    color: 'text.primary',
                    textAlign: 'left',
                  }}
                >
                  {fromAddress}
                </Typography>
                <CopyToClipboard value={fromAddress} sx={{ flexShrink: 0, mt: 0.25 }} />
              </Box>
            ) : (
              <Typography variant="body2" color="text.secondary" sx={{ textAlign: 'center' }}>
                -
              </Typography>
            )}
          </Box>
        </Box>

        {/* Recipient address field with paste button */}
        <TextFieldWithPaste
          label="Recipient address"
          fullWidth
          onChange={(e) => {
            setRecipientTouched(true);
            onAddressChange(e.target.value);
          }}
          onBlur={() => {
            if (recipientBlurReady) {
              setRecipientTouched(true);
            }
          }}
          value={toAddress}
          error={(recipientTouched && !toAddress.trim()) || (toAddress !== '' && !addressIsValid)}
          helperText={recipientHelperText(recipientTouched, toAddress, addressIsValid)}
          InputLabelProps={{ shrink: true }}
          onPasteValue={(v) => {
            setRecipientTouched(true);
            onAddressChange(v);
          }}
        />

        {/* Amount: Max in endAdornment; no paste chip (recipient/memo still have paste). */}
        <CurrencyFormFieldWithPaste
          key={`send-amount-${amountFieldKey}`}
          label="Amount"
          fullWidth
          showPaste={false}
          onChanged={(value) => {
            setAmountTouched(true);
            onAmountChange(value);
            validateSendAmount(value, true).catch(() => {
              /* validateSendAmount only updates state */
            });
          }}
          initialValue={amount?.amount}
          denom={denom}
          validationError={errorAmount}
          endAdornment={
            <Button
              variant="outlined"
              size="small"
              disabled={noAccount || !addressIsValid || maxAmountLoading}
              onClick={async (e) => {
                e.preventDefault();
                e.stopPropagation();
                await onMaxAmount();
              }}
              sx={{
                minWidth: 52,
                py: 0.25,
                px: 0.75,
                textTransform: 'none',
                fontWeight: 600,
                lineHeight: 1.2,
              }}
              aria-label="Fill maximum sendable amount after fees"
            >
              {maxAmountLoading ? <CircularProgress size={16} color="inherit" /> : 'Max'}
            </Button>
          }
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

        <Stack gap={0.5}>
          <ModalListItem
            label="Account balance"
            value={balance ? balance.toUpperCase() : '0'}
            divider
            fontWeight={600}
          />
          <Typography fontSize="smaller" sx={{ color: 'text.primary' }}>
            Fees use the current network estimate (or your custom fee if set below). Max fills your balance minus that
            fee and a small reserve. The next step shows the full breakdown before you confirm.
          </Typography>
        </Stack>

        <FormControlLabel
          control={<Checkbox onChange={() => setShowMore(!showMore)} checked={showMore} />}
          label="More options"
        />

        {showMore && (
          <Stack spacing={2}>
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
      </Stack>
    </SimpleModal>
  );
};
