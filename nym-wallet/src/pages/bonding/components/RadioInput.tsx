import * as React from 'react';
import { Control, useController } from 'react-hook-form';
import {
  FormControl,
  FormControlLabel,
  FormLabel,
  FormLabelProps,
  Radio,
  RadioGroup,
  RadioGroupProps,
  RadioProps,
} from '@mui/material';

interface Props {
  name: string;
  label: string;
  control: Control<any>;
  options: { label: string; value: any }[];
  defaultValue: any;
  muiRadioGroupProps?: RadioGroupProps;
  muiRadioProps?: RadioProps;
  muiFormLabelProps?: FormLabelProps;
}

const RadioInput = ({
  label,
  control,
  options,
  defaultValue,
  name,
  muiRadioGroupProps,
  muiRadioProps,
  muiFormLabelProps,
}: Props) => {
  const {
    field: { onChange, value, ref },
  } = useController({
    name,
    control,
    rules: { required: true },
    defaultValue,
  });
  return (
    <FormControl ref={ref}>
      <FormLabel
        id={`radio-group-label-${name}`}
        sx={{
          color: 'text.main',
        }}
        {...muiFormLabelProps}
      >
        {label}
      </FormLabel>
      <RadioGroup
        value={value}
        onChange={onChange}
        aria-labelledby={`radio-group-label-${name}`}
        name={name}
        sx={{
          color: 'text.main',
        }}
        {...muiRadioGroupProps}
      >
        {options.map(({ value: v, label: l }) => (
          <FormControlLabel key={v} value={v} control={<Radio color="default" {...muiRadioProps} />} label={l} />
        ))}
      </RadioGroup>
    </FormControl>
  );
};

export default RadioInput;
