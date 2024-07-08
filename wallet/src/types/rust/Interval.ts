export interface Interval {
  id: number;
  epochs_in_interval: number;
  current_epoch_start_unix: bigint;
  current_epoch_id: number;
  epoch_length_seconds: bigint;
  total_elapsed_epochs: number;
}
