export interface PendingUndelegate {
  mix_identity: string;
  delegate: string;
  proxy: string | null;
  block_height: bigint;
}
