import * as React from 'react';
import type { DecCoin } from '@nymproject/types';
import { Stack, SxProps, Typography } from '@mui/material';
import { CurrencyWithCoinMark } from './CurrencyWithCoinMark';
import { CURRENCY_AMOUNT_SPACING, CurrencyAmount } from './CurrencyAmount';

export const Currency: React.FC<{
  majorAmount?: DecCoin;
  showDenom?: boolean;
  showCoinMark?: boolean;
  hideFractions?: boolean;
  coinMarkPrefix?: boolean;
  sx?: SxProps;
}> = ({ majorAmount, sx, showDenom = true, showCoinMark = false, coinMarkPrefix = false, hideFractions = false }) => {
  if (!majorAmount || !majorAmount.amount) {
    return (
      <Stack direction="row" sx={sx} fontSize="inherit">
        <span>-</span>
      </Stack>
    );
  }
  if (!showDenom) {
    return <CurrencyAmount majorAmount={majorAmount} hideFractions={hideFractions} sx={sx} />;
  }
  if (showCoinMark) {
    return (
      <CurrencyWithCoinMark majorAmount={majorAmount} hideFractions={hideFractions} prefix={coinMarkPrefix} sx={sx} />
    );
  }
  return (
    <Stack direction="row" spacing={CURRENCY_AMOUNT_SPACING} sx={sx} fontSize="inherit">
      <CurrencyAmount majorAmount={majorAmount} hideFractions={hideFractions} />
      <span>{majorAmount.denom}</span>
    </Stack>
  );
};
