/* eslint-disable global-require, @typescript-eslint/no-require-imports */
import type { ComponentType } from 'react';

export type QrCodeReactProps = {
  value: string;
  size?: number;
  level?: 'L' | 'M' | 'Q' | 'H';
  includeMargin?: boolean;
  bgColor?: string;
  fgColor?: string;
  renderAs?: 'canvas' | 'svg';
  'data-testid'?: string;
};

/** qrcode.react is CJS; its package types do not match bundler default interop. */
export const QrCodeReact = require('qrcode.react') as ComponentType<QrCodeReactProps>;
