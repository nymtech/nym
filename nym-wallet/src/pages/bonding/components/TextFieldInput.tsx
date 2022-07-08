import * as React from 'react';
import { Control, useController } from 'react-hook-form';
import { SxProps, TextField, TextFieldProps } from '@mui/material';
import { RegisterOptions } from 'react-hook-form/dist/types/validator';

interface Props {
  name: string;
  label: string;
  placeholder?: string;
  control: Control<any>;
  defaultValue?: string;
  required?: boolean;
  error?: boolean;
  muiTextFieldProps?: TextFieldProps;
  helperText?: string;
  sx?: SxProps;
  registerOptions?: RegisterOptions;
  disabled?: boolean;
}

const TextFieldInput = ({
  name,
  label,
  control,
  defaultValue,
  placeholder,
  muiTextFieldProps,
  required,
  error,
  helperText,
  registerOptions,
  sx,
  disabled,
}: Props) => {
  const {
    field: { onChange, onBlur, value, ref },
  } = useController({
    name,
    control,
    defaultValue,
    rules: registerOptions,
  });
  return (
    <TextField
      onChange={onChange}
      onBlur={onBlur}
      value={value}
      name={name}
      id={name}
      label={label}
      variant="outlined"
      placeholder={placeholder}
      required={required}
      inputRef={ref}
      error={error}
      helperText={helperText}
      {...muiTextFieldProps}
      sx={sx}
      disabled={disabled}
    />
  );
};

export default TextFieldInput;
