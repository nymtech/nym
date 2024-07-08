import Wordmark from '@assets/logo/logo-wordmark.svg?react';
import { useTheme } from '@mui/material';
import { LogoProps } from './LogoProps';

type NymWordmarkProps = LogoProps & { fill?: string };

export const NymWordmark = ({ height, width, fill }: NymWordmarkProps) => {
  const theme = useTheme();
  return <Wordmark height={height} width={width} fill={fill || theme.palette.text.primary} />;
};
