import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Stack } from '@mui/material';
import { Currency } from './Currency';
import { amounts } from './CurrencyAmount.stories';

export default {
  title: 'Currency/Currency display',
  component: Currency,
} as ComponentMeta<typeof Currency>;

export const Mainnet = () => (
  <Stack direction="column">
    <Currency majorAmount={{ amount: '42.123456', denom: 'NYM' }} />
    <Currency majorAmount={{ amount: '42.123456', denom: 'NYM' }} showDenom={false} />
    <Currency majorAmount={{ amount: '42.123456', denom: 'NYM' }} showCoinMark />
    <Currency majorAmount={{ amount: '42.123456', denom: 'NYM' }} showCoinMark coinMarkPrefix />
    {amounts.map((amount) => (
      <Currency key={amount} majorAmount={{ amount, denom: 'NYM' }} showCoinMark coinMarkPrefix />
    ))}
  </Stack>
);

export const Testnet = () => (
  <Stack direction="column">
    <Currency majorAmount={{ amount: '42.123456', denom: 'NYMT' }} />
    <Currency majorAmount={{ amount: '42.123456', denom: 'NYMT' }} showDenom={false} />
    <Currency majorAmount={{ amount: '42.123456', denom: 'NYMT' }} showCoinMark />
    <Currency majorAmount={{ amount: '42.123456', denom: 'NYMT' }} showCoinMark coinMarkPrefix />
    {amounts.map((amount) => (
      <Currency key={amount} majorAmount={{ amount, denom: 'NYMT' }} showCoinMark coinMarkPrefix />
    ))}
  </Stack>
);

export const Empty = () => <Currency />;

export const WithSX = () => (
  <Stack direction="column">
    {amounts.map((amount) => (
      <Currency
        key={amount}
        majorAmount={{ amount, denom: 'NYM' }}
        showCoinMark
        sx={{ fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 }}
      />
    ))}
    {amounts.map((amount) => (
      <Currency
        key={amount}
        majorAmount={{ amount, denom: 'NYM' }}
        sx={{ fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 }}
      />
    ))}
  </Stack>
);
