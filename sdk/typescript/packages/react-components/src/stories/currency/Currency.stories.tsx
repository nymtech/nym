import { ComponentMeta } from '@storybook/react';
import { Stack } from '@mui/material';
import { Currency } from '@lib/components/currency';
import { amounts } from './CurrencyAmount.stories';

export default {
  title: 'Currency/Currency display',
  component: Currency,
} as ComponentMeta<typeof Currency>;

export const Mainnet = () => (
  <Stack direction="column">
    <Currency majorAmount={{ amount: '42.123456', denom: 'nym' }} />
    <Currency majorAmount={{ amount: '42.123456', denom: 'nym' }} showDenom={false} />
    <Currency majorAmount={{ amount: '42.123456', denom: 'nym' }} showCoinMark />
    <Currency majorAmount={{ amount: '42.123456', denom: 'nym' }} showCoinMark coinMarkPrefix />
    {amounts.map((amount) => (
      <Currency key={amount} majorAmount={{ amount, denom: 'nym' }} showCoinMark coinMarkPrefix />
    ))}
  </Stack>
);

export const Testnet = () => (
  <Stack direction="column">
    <Currency majorAmount={{ amount: '42.123456', denom: 'nymt' }} />
    <Currency majorAmount={{ amount: '42.123456', denom: 'nymt' }} showDenom={false} />
    <Currency majorAmount={{ amount: '42.123456', denom: 'nymt' }} showCoinMark />
    <Currency majorAmount={{ amount: '42.123456', denom: 'nymt' }} showCoinMark coinMarkPrefix />
    {amounts.map((amount) => (
      <Currency key={amount} majorAmount={{ amount, denom: 'nymt' }} showCoinMark coinMarkPrefix />
    ))}
  </Stack>
);

export const Empty = () => <Currency />;

export const WithSX = () => (
  <Stack direction="column">
    {amounts.map((amount) => (
      <Currency
        key={amount}
        majorAmount={{ amount, denom: 'nym' }}
        showCoinMark
        sx={{ fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 }}
      />
    ))}
    {amounts.map((amount) => (
      <Currency
        key={amount}
        majorAmount={{ amount, denom: 'nym' }}
        sx={{ fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 }}
      />
    ))}
  </Stack>
);
