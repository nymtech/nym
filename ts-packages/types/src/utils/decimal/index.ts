/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

import { Decimal } from '@cosmjs/math';

export const stringToDecimal = (raw: string): Decimal => Decimal.fromUserInput(raw, 0);

export const decimalToPercentage = (raw: string) =>
  Math.round(Decimal.fromUserInput(raw, 18).toFloatApproximation() * 100).toString();

export const decimalToFloatApproximation = (raw: string): number =>
  Decimal.fromUserInput(raw, 18).toFloatApproximation();
