export type Detailed = {
    initial_supply: InitialSupply;
    mixmining_reserve: MixminingReserve;
    vesting_tokens: VestingTokens;
    circulating_supply: CirculatingSupply;
  };
  
  export type InitialSupply = {
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
