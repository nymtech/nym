import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Stack, Typography } from '@mui/material';
import { CoinMark } from './CoinMark';
import { CoinMarkTestnet } from './CoinMarkTestnet';

export default {
  title: 'Branding/Coin Mark (testnet)',
  component: CoinMarkTestnet,
} as ComponentMeta<typeof CoinMarkTestnet>;

export const Auto = () => <CoinMarkTestnet height={250} />;

export const LightMode = () => <CoinMarkTestnet mode="light" height={250} />;

export const DarkMode = () => <CoinMarkTestnet mode="dark" height={250} />;

const sizes = [8, 10, 12, 16, 20, 32, 40, 64];

export const Sizes = () => (
  <Stack direction="column" spacing={2}>
    {sizes.map((size) => (
      <Stack direction="row" spacing={4} p={1} alignItems="center" borderBottom="1px solid #444">
        <Typography sx={{ opacity: 0.5 }} width="40px">
          {size}px
        </Typography>
        <CoinMarkTestnet key={size} height={size} />
      </Stack>
    ))}
  </Stack>
);
