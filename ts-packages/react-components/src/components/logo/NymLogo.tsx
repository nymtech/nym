import * as React from 'react';
import Logo from '@assets/logo/logo-circle.svg';
import { LogoProps } from './LogoProps';

export const NymLogo: React.FC<LogoProps> = ({ height, width }) => <Logo height={height} width={width} />;
