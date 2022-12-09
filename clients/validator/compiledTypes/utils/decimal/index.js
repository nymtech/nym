/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */
import { Decimal } from '@cosmjs/math';
export const stringToDecimal = (raw) => Decimal.fromUserInput(raw, 0);
export const decimalToPercentage = (raw) => Math.round(Decimal.fromUserInput(raw, 18).toFloatApproximation() * 100).toString();
export const decimalToFloatApproximation = (raw) => Decimal.fromUserInput(raw, 18).toFloatApproximation();
//# sourceMappingURL=index.js.map