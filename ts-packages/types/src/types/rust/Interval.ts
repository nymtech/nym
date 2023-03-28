export interface Interval {
  id: number;
  epochs_in_interval: number;
  current_epoch_start: string;
  current_epoch_id: number;
  epoch_length: { secs: number; nanos: number };
  total_elapsed_epochs: number;
}
