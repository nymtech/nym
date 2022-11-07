import type {PendingIntervalEventData} from "./PendingIntervalEventData";

export interface PendingIntervalEvent {
  id: number,
  created_at: bigint,
  event: PendingIntervalEventData,
}
