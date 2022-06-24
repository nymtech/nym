import * as React from 'react';
import { Control, useController } from 'react-hook-form';
import { Checkbox, CheckboxProps, FormControlLabel, FormControlLabelProps, FormGroup, SxProps } from '@mui/material';

interface Props {
  name: string;
  label: string;
  control: Control<any>;
  defaultValue: boolean;
  muiCheckboxProps?: CheckboxProps;
  muiFormControlLabelProps?: FormControlLabelProps;
  sx?: SxProps;
}

const CheckboxInput = ({
  name,
  control,
  defaultValue,
  label,
  muiCheckboxProps,
  muiFormControlLabelProps,
  sx,
}: Props) => {
  const {
    field: { onChange, onBlur, value, ref },
  } = useController({
    name,
    control,
    defaultValue,
  });
  return (
    <FormGroup sx={sx}>
      <FormControlLabel
        control={
          <Checkbox
            onBlur={onBlur}
            onChange={onChange}
            checked={value}
            inputRef={ref}
            name={name}
            {...muiCheckboxProps}
          />
        }
        label={label}
        {...muiFormControlLabelProps}
      />
    </FormGroup>
  );
};

export default CheckboxInput;
