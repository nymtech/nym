/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.35.3.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export interface InstantiateMsg {
  mix_denom: string;
  multisig_addr: string;
  pool_addr: string;
}
export type ExecuteMsg = {
  deposit_funds: {
    data: DepositData;
  };
} | {
  spend_credential: {
    data: SpendCredentialData;
  };
} | {
  release_funds: {
    funds: Coin;
  };
};
export type Uint128 = string;
export interface DepositData {
  deposit_info: string;
  encryption_key: string;
  identity_key: string;
}
export interface SpendCredentialData {
  blinded_serial_number: string;
  funds: Coin;
  gateway_cosmos_address: string;
}
export interface Coin {
  amount: Uint128;
  denom: string;
  [k: string]: unknown;
}
export type QueryMsg = {
  get_spent_credential: {
    blinded_serial_number: string;
  };
} | {
  get_all_spent_credentials: {
    limit?: number | null;
    start_after?: string | null;
  };
};
export interface MigrateMsg {}
export type Addr = string;
export type SpendCredentialStatus = "in_progress" | "spent";
export interface PagedSpendCredentialResponse {
  per_page: number;
  spend_credentials: SpendCredential[];
  start_next_after?: string | null;
}
export interface SpendCredential {
  blinded_serial_number: string;
  funds: Coin;
  gateway_cosmos_address: Addr;
  status: SpendCredentialStatus;
}
export interface SpendCredentialResponse {
  spend_credential?: SpendCredential | null;
}