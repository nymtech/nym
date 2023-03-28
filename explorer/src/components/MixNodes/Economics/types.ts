export type EconomicsRowsType = {
  progressBarValue?: number;
  value: string;
};

type TEconomicsInfoProperties =
  | 'estimatedTotalReward'
  | 'estimatedOperatorReward'
  | 'estimatedOperatorReward'
  | 'selectionChance'
  | 'profitMargin'
  | 'avgUptime'
  | 'nodePerformance'
  | 'operatingCost';

export type EconomicsInfoRow = {
  [k in TEconomicsInfoProperties]: EconomicsRowsType;
};

export type EconomicsInfoRowWithIndex = EconomicsInfoRow & { id: number };
