export type EconomicsRowsType = {
  progressBarValue?: number;
  value?: string;
};

export interface EconomicsInfoRow {
  estimated_total_reward: EconomicsRowsType;
  estimated_operator_reward: EconomicsRowsType;
  active_set_probability: EconomicsRowsType;
  stake_saturation: EconomicsRowsType;
  profit_margin: EconomicsRowsType;
  avg_uptime: EconomicsRowsType;
}

export type EconomicsInfoRowWithIndex = EconomicsInfoRow & { id: number };
