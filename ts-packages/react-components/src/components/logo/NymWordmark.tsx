import * as React from 'react';
import Wordmark from '@assets/logo/logo-wordmark.svg';
import { useTheme } from '@mui/material';
import { LogoProps } from './LogoProps';

export const NymWordmark: React.FC<LogoProps & { fill?: string }> = ({ height, width, fill }) => {
  const theme = useTheme();
  return <Wordmark height={height} width={width} fill={fill || theme.palette.text.primary} />;
};
