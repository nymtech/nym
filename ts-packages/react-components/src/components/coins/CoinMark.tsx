import * as React from 'react';
import { PaletteMode, useTheme } from '@mui/material';
import TokenLight from '@assets/token/token-light.svg';
import TokenDark from '@assets/token/token-dark.svg';

export const CoinMark: React.FC<{
  mode?: PaletteMode;
  width?: number | string;
  height?: number | string;
}> = ({ mode, ...props }) => {
  const theme = useTheme();
  const modeWithTheme = mode || theme.palette.mode;
  if (modeWithTheme === 'light') {
    return <TokenLight {...props} />;
  }
  return <TokenDark {...props} />;
};
