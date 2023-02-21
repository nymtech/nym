import * as React from 'react';
import Logo from '@assets/logo/logo-circle-small.svg';
import { LogoProps } from './LogoProps';

export const NymIcon: React.FC<LogoProps> = ({ height, width }) => <Logo height={height} width={width} />;
