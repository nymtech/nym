import * as React from 'react';
import Wordmark from '@assets/logo/logo-wordmark.svg';
import { LogoProps } from './LogoProps';

export const NymWordmark: React.FC<LogoProps> = ({ height, width }) => <Wordmark height={height} width={width} />;
