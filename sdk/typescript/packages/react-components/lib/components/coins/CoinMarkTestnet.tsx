import { useTheme } from '@mui/material';
import TokenLight from '@assets/token/token-light-testnet.svg?react';
import TokenDark from '@assets/token/token-dark-testnet.svg?react';
import { CoinMarkProps } from './CoinMark';

export const CoinMarkTestnet = ({ mode, ...props }: CoinMarkProps) => {
  const theme = useTheme();
  const modeWithTheme = mode || theme.palette.mode;
  if (modeWithTheme === 'light') {
    return <TokenLight {...props} />;
  }
  return <TokenDark {...props} />;
};
