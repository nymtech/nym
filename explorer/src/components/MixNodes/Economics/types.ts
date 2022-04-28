export type EconomicsRowsType = {
  progressBarValue?: number;
  value: string;
};

export interface EconomicsInfoRow {
  estimatedTotalReward: EconomicsRowsType;
  estimatedOperatorReward: EconomicsRowsType;
  selectionChance: EconomicsRowsType;
  stakeSaturation: EconomicsRowsType;
  profitMargin: EconomicsRowsType;
  avgUptime: EconomicsRowsType;
}

export type EconomicsInfoRowWithIndex = EconomicsInfoRow & { id: number };
