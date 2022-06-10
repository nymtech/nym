import * as React from 'react';
import { ChangeEvent } from 'react';
import { InputAdornment, TextField } from '@mui/material';
import { TextFieldProps } from '@mui/material/TextField/TextField';
import { validateKey } from '@nymproject/types';
import DoneIcon from '@mui/icons-material/Done';
import { SxProps } from '@mui/system';

export const IdentityKeyFormField: React.FC<{
  showTickOnValid?: boolean;
  fullWidth?: boolean;
  required?: boolean;
  readOnly?: boolean;
  initialValue?: string;
  placeholder?: string;
  onChanged?: (newValue: string) => void;
  onValidate?: (isValid: boolean, error?: string) => void;
  textFieldProps?: TextFieldProps;
  sx?: SxProps;
  saturation?: number;
}> = ({
  required,
  fullWidth,
  placeholder,
  readOnly,
  initialValue,
  sx,
  onChanged,
  onValidate,
  textFieldProps,
  showTickOnValid = true,
  saturation,
}) => {
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

    if (newValue === undefined && saturation && saturation > 100) {
      const newSaturationError = `This node is over saturated (${saturation}%), please select another one`;
      setValidationError(newSaturationError);
      if (onValidate) {
        onValidate(false, newSaturationError);
      }
      return false;
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
  }, [initialValue]);

  React.useEffect(() => {
    // check if the node is over saturated
    if (saturation) {
      doValidation();
    }
  }, [saturation]);

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const newValue = event.target.value;
    if (doValidation(newValue)) {
      setValue(newValue);
      if (onChanged) {
        onChanged(newValue);
      }
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
      sx={sx}
      {...textFieldProps}
      aria-readonly={readOnly}
      error={validationError !== undefined}
      helperText={validationError}
      defaultValue={initialValue}
      onChange={handleChange}
    />
  );
};
