import { ComponentMeta } from '@storybook/react';
import { Stack } from '@mui/material';
import { CurrencyFormField } from '@lib/components/currency';

export default {
  title: 'Currency/Currency form field',
  component: CurrencyFormField,
} as ComponentMeta<typeof CurrencyFormField>;

export const Mainnet = () => <CurrencyFormField initialValue="42.123456" denom="nymt" />;

export const Testnet = () => <CurrencyFormField initialValue="42.123456" denom="nymt" />;

export const FullWidth = () => <CurrencyFormField initialValue="42.123456" denom="nym" fullWidth />;

export const HideCoinMark = () => <CurrencyFormField initialValue="42.123456" denom="nym" showCoinMark={false} />;

export const ErrorToBig = () => <CurrencyFormField initialValue="1_000_000_000_000_001" denom="nym" />;

export const ErrorToSmall = () => <CurrencyFormField initialValue="0.0000001" denom="nym" />;

export const ErrorNegative = () => <CurrencyFormField initialValue="-1" denom="nym" />;

export const MicroNym = () => <CurrencyFormField initialValue="0.000001" denom="nym" />;

export const Zero = () => <CurrencyFormField initialValue="0" denom="nym" />;

export const Empty = () => <CurrencyFormField />;

export const EmptyWithAutofocus = () => <CurrencyFormField autoFocus />;

export const ReadOnly = () => (
  <Stack direction="column" spacing={2}>
    <CurrencyFormField initialValue="42.123456" denom="nym" readOnly />
    <CurrencyFormField initialValue="42.123456" denom="nymt" readOnly />
  </Stack>
);
