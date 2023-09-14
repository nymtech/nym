import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Stack, Typography } from '@mui/material';
import { CoinMark } from './CoinMark';

export default {
  title: 'Branding/Coin Mark',
  component: CoinMark,
} as ComponentMeta<typeof CoinMark>;

export const Auto = () => <CoinMark height={250} />;

export const LightMode = () => <CoinMark mode="light" height={250} />;

export const DarkMode = () => <CoinMark mode="dark" height={250} />;

const sizes = [8, 10, 12, 16, 20, 32, 40, 64];

export const Sizes = () => (
  <Stack direction="column" spacing={2}>
    {sizes.map((size) => (
      <Stack direction="row" spacing={4} p={1} alignItems="center" borderBottom="1px solid #444">
        <Typography sx={{ opacity: 0.5 }} width="40px">
          {size}px
        </Typography>
        <CoinMark key={size} height={size} />
      </Stack>
    ))}
  </Stack>
);
