import { PaletteMode, useTheme } from '@mui/material';
import TokenLight from '@assets/token/token-light.svg?react';
import TokenDark from '@assets/token/token-dark.svg?react';

export type CoinMarkProps = {
  mode?: PaletteMode;
  width?: number | string;
  height?: number | string;
};

export const CoinMark = ({ mode, ...props }: CoinMarkProps) => {
  const theme = useTheme();
  const modeWithTheme = mode || theme.palette.mode;
  if (modeWithTheme === 'light') {
    return <TokenLight {...props} />;
  }
  return <TokenDark {...props} />;
};
