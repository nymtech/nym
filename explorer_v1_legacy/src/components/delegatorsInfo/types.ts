export type RowsType = {
  value?: string | number;
  visualProgressValue?: number;
};

export interface DelegatorsInfoRow {
  estimated_total_reward: RowsType;
  estimated_operator_reward: RowsType;
  active_set_probability: RowsType;
  stake_saturation: RowsType;
  profit_margin: RowsType;
  avg_uptime: RowsType;
}

export type DelegatorsInfoRowWithIndex = DelegatorsInfoRow & { id: number };
