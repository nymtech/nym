import type { CosmosFee } from './CosmosFee';

export type Fee = { Manual: CosmosFee } | { Auto: number | null };
