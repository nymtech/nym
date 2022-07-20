import * as React from 'react';
import { Control, useController } from 'react-hook-form';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';

interface Props {
  name: string;
  label: string;
  control: Control<any>;
  required?: boolean;
  fullWidth?: boolean;
  errorMessage?: string;
  currencyDenom?: string;
}

const CurrencyInput = ({ name, label, control, errorMessage, currencyDenom, required, fullWidth }: Props) => {
  const {
    field: { onChange },
  } = useController({
    name,
    control,
  });

  return (
    <CurrencyFormField
      showCoinMark
      required={required}
      fullWidth={fullWidth}
      label={label}
      onChanged={onChange}
      denom={currencyDenom}
      validationError={errorMessage}
    />
  );
};

export default CurrencyInput;
