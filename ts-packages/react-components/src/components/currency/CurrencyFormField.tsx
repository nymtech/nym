import * as React from 'react';
import { ChangeEvent } from 'react';
import { InputAdornment, TextField } from '@mui/material';
import { SxProps } from '@mui/system';
import { CurrencyDenom, MajorCurrencyAmount } from '@nymproject/types';
import { CoinMark } from '../coins/CoinMark';

const MAX_VALUE = 1_000_000_000_000_000;
const MIN_VALUE = 0.000001;

export const CurrencyFormField: React.FC<{
  autoFocus?: boolean;
  required?: boolean;
  fullWidth?: boolean;
  readOnly?: boolean;
  showCoinMark?: boolean;
  initialValue?: string;
  validationError?: string;
  placeholder?: string;
  label?: string;
  denom?: CurrencyDenom;
  onChanged?: (newValue: MajorCurrencyAmount) => void;
  onValidate?: (newValue: string | undefined, isValid: boolean, error?: string) => void;
  sx?: SxProps;
}> = ({
  autoFocus,
  required,
  placeholder,
  fullWidth,
  readOnly,
  initialValue,
  validationError: validationErrorProp,
  label,
  onChanged,
  onValidate,
  sx,
  showCoinMark = true,
  denom = 'NYM',
}) => {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [value, setValue] = React.useState<string | undefined>(initialValue);
  const [validationError, setValidationError] = React.useState<string | undefined>(validationErrorProp);

  React.useEffect(() => {
    setValidationError(validationErrorProp);
  }, [validationErrorProp]);

  const fireOnValidate = (result: boolean) => {
    if (onValidate) {
      onValidate(value, result);
    }
    return result;
  };

  const doValidation = (newValue?: string): boolean => {
    // the external validation error is set, so it overrides internal validation messages
    if (validationErrorProp) {
      setValidationError(validationErrorProp);
      return false;
    }

    // handle empty value
    if (!newValue) {
      setValue(undefined);
      setValidationError(undefined);
      return fireOnValidate(false);
    }

    try {
      const numberAsString = (newValue || '0').trim();
      const newNumber = Number.parseFloat(numberAsString);

      // no negative numbers
      if (newNumber < 0) {
        setValidationError('Amount cannot be negative');
        return fireOnValidate(false);
      }

      // it cannot be larger than the total supply
      if (newNumber > MAX_VALUE) {
        setValidationError('Amount cannot be bigger than the total supply of NYMs');
        return fireOnValidate(false);
      }

      // it can't be lower than one micro coin
      if (newNumber < MIN_VALUE) {
        setValidationError('Amount cannot be less than 1 uNYM');
        return fireOnValidate(false);
      }

      setValidationError(undefined);
      setValue(numberAsString);

      return fireOnValidate(true);
    } catch (e) {
      setValidationError((e as Error).message);
      return fireOnValidate(false);
    }
  };

  React.useEffect(() => {
    // validate initial value (only if set), so that validation error UI hints are set without the user typing
    if (initialValue) {
      doValidation(initialValue);
    }
  }, [initialValue]);

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const newValue: string | undefined = event.target.value?.trim();

    doValidation(newValue);

    if (onChanged) {
      const newMajorCurrencyAmount: MajorCurrencyAmount = {
        amount: newValue,
        denom,
      };
      onChanged(newMajorCurrencyAmount);
    }
  };

  return (
    <TextField
      // see https://technology.blog.gov.uk/2020/02/24/why-the-gov-uk-design-system-team-changed-the-input-type-for-numbers/
      // for more information about entering numbers in form fields
      type="text"
      inputMode="numeric"
      autoFocus={autoFocus}
      fullWidth={fullWidth}
      InputProps={{
        readOnly,
        required,
        endAdornment: showCoinMark && (
          <InputAdornment position="end">
            {denom === 'NYM' && <CoinMark height="20px" />}
            {denom !== 'NYM' && <span>NYMT</span>}
          </InputAdornment>
        ),
        ...{
          min: MIN_VALUE,
          max: MAX_VALUE,
        },
      }}
      aria-readonly={readOnly}
      error={validationError !== undefined}
      helperText={validationError}
      defaultValue={initialValue}
      placeholder={placeholder}
      label={label}
      onChange={handleChange}
      sx={sx}
    />
  );
};
