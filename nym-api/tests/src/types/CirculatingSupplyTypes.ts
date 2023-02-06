export type Detailed = {
  total_supply: TotalSupply;
  mixmining_reserve: MixminingReserve;
  vesting_tokens: VestingTokens;
  circulating_supply: CirculatingSupply;
};

export type TotalSupply = {
  demon: "unym";
  amount: string;
};

export type MixminingReserve = {
  demon: "unym";
  amount: string;
};

export type VestingTokens = {
  demon: "unym";
  amount: string;
};

export type CirculatingSupply = {
  demon: "unym";
  amount: string;
};
