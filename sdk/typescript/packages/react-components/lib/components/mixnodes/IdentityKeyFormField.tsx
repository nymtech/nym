import * as React from 'react';
import { ChangeEvent } from 'react';
import { InputAdornment, TextField, SxProps } from '@mui/material';
import { TextFieldProps } from '@mui/material/TextField/TextField';
import { validateKey } from '@nymproject/types';
import DoneIcon from '@mui/icons-material/Done';

export type IdentityKeyFormFieldProps = {
  showTickOnValid?: boolean;
  fullWidth?: boolean;
  required?: boolean;
  readOnly?: boolean;
  initialValue?: string;
  placeholder?: string;
  label?: string;
  helperText?: string;
  onChanged?: (newValue: string) => void;
  onValidate?: (isValid: boolean, error?: string) => void;
  textFieldProps?: TextFieldProps;
  errorText?: string;
  size?: 'small' | 'medium';
  sx?: SxProps;
  disabled?: boolean;
  autoFocus?: boolean;
};

export const IdentityKeyFormField = ({
  required,
  fullWidth,
  placeholder,
  label,
  readOnly,
  initialValue,
  errorText,
  sx,
  onChanged,
  onValidate,
  textFieldProps,
  showTickOnValid = true,
  size,
  disabled,
  autoFocus,
}: IdentityKeyFormFieldProps) => {
  const [value, setValue] = React.useState<string | undefined>(initialValue);
  const [validationError, setValidationError] = React.useState<string | undefined>();

  const doValidation = (newValue?: string): boolean => {
    if (validateKey(newValue)) {
      setValidationError(undefined);
      if (onValidate) {
        onValidate(true);
      }
      return true;
    }

    const newValidationError = 'Key is not valid';
    setValidationError(newValidationError);
    if (onValidate) {
      onValidate(false, newValidationError);
    }

    return false;
  };

  React.useEffect(() => {
    // validate initial value (only if set), so that validation error UI hints are set without the user typing
    if (initialValue) {
      doValidation(initialValue);
    }

    if (errorText) {
      setValidationError(errorText);
    }
  }, [initialValue, errorText]);

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const newValue = event.target.value;

    if (doValidation(newValue)) {
      setValue(newValue);
    }

    if (onChanged) {
      onChanged(newValue);
    }
  };

  return (
    <TextField
      fullWidth={fullWidth}
      InputProps={{
        readOnly,
        required,
        endAdornment: showTickOnValid && value && validationError === undefined && (
          <InputAdornment position="end">
            <DoneIcon color="success" />
          </InputAdornment>
        ),
      }}
      placeholder={placeholder}
      label={label}
      sx={sx}
      {...textFieldProps}
      aria-readonly={readOnly}
      error={validationError !== undefined}
      helperText={validationError}
      defaultValue={initialValue}
      onChange={handleChange}
      InputLabelProps={{ shrink: true }}
      size={size}
      disabled={disabled}
      autoFocus={autoFocus}
    />
  );
};
