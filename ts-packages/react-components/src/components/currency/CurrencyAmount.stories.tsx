import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Stack } from '@mui/material';
import { CurrencyAmount } from './CurrencyAmount';

export default {
  title: 'Currency/Currency amount',
  component: CurrencyAmount,
} as ComponentMeta<typeof CurrencyAmount>;

export const amounts = [
  undefined,
  '0',
  '0.1',
  '0.01',
  '0.001',
  '0.0001',
  '0.00001',
  '1.000001',
  '10.000001',
  '100.000001',
  '1000.000001',
  '10000.000001',
  '100000.000001',
  '1000000.000001',
  '10000000.000001',
  '100000000.000001',
  '1000000000.000001',
  '10000000000.000001',
  '100000000000.000001',
  '1000000000000.000001',
];

export const WithSeparators = () => (
  <Stack direction="column">
    {amounts.map((amount) => (
      <CurrencyAmount key={amount} majorAmount={{ amount, denom: 'NYM' }} />
    ))}
  </Stack>
);

export const NoSeparators = () => (
  <Stack direction="column">
    {amounts.map((amount) => (
      <CurrencyAmount key={amount} majorAmount={{ amount, denom: 'NYM' }} showSeparators={false} />
    ))}
  </Stack>
);

export const MaxRange = () => <CurrencyAmount majorAmount={{ amount: '1000000000000.000001', denom: 'NYM' }} />;

export const Weird = () => (
  <Stack direction="column">
    <CurrencyAmount majorAmount={{ amount: '0000000000000.000000', denom: 'NYM' }} />
    <CurrencyAmount majorAmount={{ amount: '0000000000000.00', denom: 'NYM' }} />
    <CurrencyAmount majorAmount={{ amount: '0000.0000', denom: 'NYM' }} />
    <CurrencyAmount majorAmount={{ amount: '0000.000', denom: 'NYM' }} />
    <CurrencyAmount majorAmount={{ amount: '0.00', denom: 'NYM' }} />
  </Stack>
);

export const Empty = () => <CurrencyAmount />;

export const NoSeparatorsWithSX = () => (
  <Stack direction="column">
    {amounts.map((amount) => (
      <CurrencyAmount
        key={amount}
        majorAmount={{ amount, denom: 'NYM' }}
        showSeparators={false}
        sx={{ fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 }}
      />
    ))}
  </Stack>
);

export const WithSX = () => (
  <Stack direction="column">
    {amounts.map((amount) => (
      <CurrencyAmount
        key={amount}
        majorAmount={{ amount, denom: 'NYM' }}
        sx={{ fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 }}
      />
    ))}
  </Stack>
);
