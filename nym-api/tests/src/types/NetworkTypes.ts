export interface NetworkDetails {
    connected_nyxd: string;
    network: Network;
}

export interface Network {
    network_name: string;
    chain_details: ChainDetails;
    endpoints: Endpoint[];
    contracts: Contracts;
    explorer_api: string;
}

export interface ChainDetails {
    bech32_account_prefix: string;
    mix_denom: Denom;
    stake_denom: Denom;
}

export interface Denom {
    base: string;
    display: string;
    display_exponent: number;
}

export interface Contracts {
    mixnet_contract_address: string;
    vesting_contract_address: string;
    coconut_bandwidth_contract_address: string;
    group_contract_address: string;
    multisig_contract_address: string;
    coconut_dkg_contract_address: string;
}

export interface Endpoint {
    nyxd_url: string;
    api_url: string;
}


export interface NymContracts {
    [additionalProp: string]: AdditionalProp;
}

export interface AdditionalProp {
    address: string;
    details: Info;
}

export interface Info {
    contract: string;
    version:  string;
}


export interface NymContractsDetailed {
    [additionalProp: string]: AdditionalPropDetailed;
}

export interface AdditionalPropDetailed {
    address: string;
    details: InfoDetailed;
}

export interface InfoDetailed {
    build_timestamp:  string;
    build_version:    string;
    commit_sha:       string;
    commit_timestamp: string;
    commit_branch:    string;
    rustc_version:    string;
}