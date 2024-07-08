/// <reference types="vite-plugin-svgr/client" />
import Logo from '@assets/logo/logo-circle-small.svg?react';
import { LogoProps } from './LogoProps';

export const NymIcon = ({ height, width }: LogoProps) => <Logo height={height} width={width} />;
