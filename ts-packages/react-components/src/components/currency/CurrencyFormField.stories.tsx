import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Stack } from '@mui/material';
import { CurrencyFormField } from './CurrencyFormField';

export default {
  title: 'Currency/Currency form field',
  component: CurrencyFormField,
} as ComponentMeta<typeof CurrencyFormField>;

export const Mainnet = () => <CurrencyFormField initialValue="42.123456" denom="NYM" />;

export const Testnet = () => <CurrencyFormField initialValue="42.123456" denom="NYMT" />;

export const FullWidth = () => <CurrencyFormField initialValue="42.123456" denom="NYM" fullWidth />;

export const HideCoinMark = () => <CurrencyFormField initialValue="42.123456" denom="NYM" showCoinMark={false} />;

export const ErrorToBig = () => <CurrencyFormField initialValue="1_000_000_000_000_001" denom="NYM" />;

export const ErrorToSmall = () => <CurrencyFormField initialValue="0.0000001" denom="NYM" />;

export const ErrorNegative = () => <CurrencyFormField initialValue="-1" denom="NYM" />;

export const MicroNym = () => <CurrencyFormField initialValue="0.000001" denom="NYM" />;

export const Zero = () => <CurrencyFormField initialValue="0" denom="NYM" />;

export const Empty = () => <CurrencyFormField />;

export const EmptyWithAutofocus = () => <CurrencyFormField autoFocus />;

export const ReadOnly = () => (
  <Stack direction="column" spacing={2}>
    <CurrencyFormField initialValue="42.123456" denom="NYM" readOnly />
    <CurrencyFormField initialValue="42.123456" denom="NYMT" readOnly />
  </Stack>
);
