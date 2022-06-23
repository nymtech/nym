import type { Fee } from "./Fee";
import type { MajorCurrencyAmount } from "./Currency";

export type FeeDetails = { amount: MajorCurrencyAmount | null, fee: Fee };