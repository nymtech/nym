// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { DecCoin } from "./DecCoin";
import type { DelegationEventKind } from "./DelegationEventKind";

export type DelegationEvent = { kind: DelegationEventKind, mix_id: number, address: string, amount: DecCoin | null, proxy: string | null, };
