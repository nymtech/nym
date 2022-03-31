import * as React from 'react';
import Wordmark from '@assets/logo/logo-wordmark.svg';
import { useTheme } from '@mui/material';
import { LogoProps } from './LogoProps';

export const NymWordmark: React.FC<LogoProps> = ({ height, width }) => {
  const theme = useTheme();
  return <Wordmark height={height} width={width} fill={theme.palette.text.primary} />;
};
