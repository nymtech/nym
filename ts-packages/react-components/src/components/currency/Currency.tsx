import * as React from 'react';
import type { MajorCurrencyAmount } from '@nymproject/types';
import { Stack, SxProps, Typography } from '@mui/material';
import { CurrencyWithCoinMark } from './CurrencyWithCoinMark';
import { CURRENCY_AMOUNT_SPACING, CurrencyAmount } from './CurrencyAmount';

export const Currency: React.FC<{
  majorAmount?: MajorCurrencyAmount;
  showDenom?: boolean;
  showCoinMark?: boolean;
  coinMarkPrefix?: boolean;
  sx?: SxProps;
}> = ({ majorAmount, sx, showDenom = true, showCoinMark = false, coinMarkPrefix = false }) => {
  if (!majorAmount || !majorAmount.amount) {
    return (
      <Stack direction="row" sx={sx}>
        <span>-</span>
      </Stack>
    );
  }
  if (!showDenom) {
    return <CurrencyAmount majorAmount={majorAmount} sx={sx} />;
  }
  if (showCoinMark) {
    return <CurrencyWithCoinMark majorAmount={majorAmount} prefix={coinMarkPrefix} sx={sx} />;
  }
  return (
    <Stack direction="row" spacing={CURRENCY_AMOUNT_SPACING} sx={sx}>
      <CurrencyAmount majorAmount={majorAmount} />
      <span>{majorAmount.denom}</span>
    </Stack>
  );
};
