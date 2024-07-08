import { DecCoin } from '@nymproject/types';
import { Stack, SxProps } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { CoinMark } from '../coins/CoinMark';
import { CoinMarkTestnet } from '../coins/CoinMarkTestnet';
import { CurrencyAmount } from './CurrencyAmount';

export type CurrencyWithCoinMarkProps = {
  majorAmount?: DecCoin;
  fontSize?: number;
  prefix?: boolean;
  showSeparators?: boolean;
  sx?: SxProps;
};

export const CurrencyWithCoinMark = ({
  majorAmount,
  fontSize,
  prefix,
  showSeparators,
  sx,
}: CurrencyWithCoinMarkProps) => {
  const theme = useTheme();
  const size = fontSize || theme.typography.htmlFontSize;
  if (!majorAmount) {
    return <span>-</span>;
  }
  const DenomMark = majorAmount.denom === 'nymt' ? CoinMarkTestnet : CoinMark;
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
