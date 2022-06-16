import * as React from 'react';
import { MajorCurrencyAmount } from '@nymproject/types';
import { Stack, SxProps } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { CoinMark } from '../coins/CoinMark';
import { CoinMarkTestnet } from '../coins/CoinMarkTestnet';
import { CurrencyAmount } from './CurrencyAmount';

export const CurrencyWithCoinMark: React.FC<{
  majorAmount?: MajorCurrencyAmount;
  fontSize?: number;
  prefix?: boolean;
  showSeparators?: boolean;
  sx?: SxProps;
}> = ({ majorAmount, fontSize, prefix, showSeparators, sx }) => {
  const theme = useTheme();
  const size = fontSize || theme.typography.htmlFontSize;
  if (!majorAmount) {
    return <span>-</span>;
  }
  const DenomMark = majorAmount.denom === 'NYMT' ? CoinMarkTestnet : CoinMark;
  return (
    <Stack direction="row" fontSize={size} spacing={1} alignItems="center" sx={sx}>
      {prefix ? (
        <>
          <DenomMark height={size} />
          <CurrencyAmount majorAmount={majorAmount} showSeparators={showSeparators} />
        </>
      ) : (
        <>
          <CurrencyAmount majorAmount={majorAmount} showSeparators={showSeparators} />
          <DenomMark height={size} />
        </>
      )}
    </Stack>
  );
};
