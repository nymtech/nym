/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

import {Decimal} from "@cosmjs/math";

export const stringToDecimal(raw: string): Decimal | null => {
  try {
    return Decimal.fromUserInput(raw, 0)
  } catch (err) {
    console.log(`${raw} is not a valid decimal - ${err}`)
    return null
  }
}
