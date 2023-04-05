import * as React from 'react';
import Logo from '@assets/logo/logo-bw.svg';
import { LogoProps } from './LogoProps';

export const NymLogoBW: FCWithChildren<LogoProps> = ({ height, width }) => <Logo height={height} width={width} />;
