/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

import {NymQuery} from "./query-client";
import {
     MixnetContractVersion
} from "./types";
import {JsonObject} from "@cosmjs/cosmwasm-stargate/build/queries";

interface SmartContractQuery {
    queryContractSmart(address: string, queryMsg: Record<string, unknown>): Promise<JsonObject>;
}

export default class NymQuerier implements NymQuery {
    client: SmartContractQuery
    constructor(client: SmartContractQuery) {
        this.client = client
    }


    getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
        return this.client.queryContractSmart(mixnetContractAddress, { get_contract_version: {  } });
    }

    //
    // public getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedMixnodeResponse> {
    //     return this.client.queryContractSmart(contractAddress, { get_mixnodes: { limit, start_after } });
    // }
    //
    // public getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse> {
    //     return this.client.queryContractSmart(contractAddress, { get_gateways: { limit, start_after } });
    // }
    //
    // public getMixDelegations(contractAddress: string, mixIdentity: string, limit: number, start_after?: string): Promise<PagedMixDelegationsResponse> {
    //     return this.client.queryContractSmart(contractAddress, {
    //         get_mix_delegations: {
    //             mix_identity: mixIdentity,
    //             limit,
    //             start_after
    //         }
    //     });
    // }
    //
    // public getMixDelegation(contractAddress: string, mixIdentity: string, delegatorAddress: string): Promise<Delegation> {
    //     return this.client.queryContractSmart(contractAddress, {
    //         get_mix_delegation: {
    //             mix_identity: mixIdentity,
    //             address: delegatorAddress
    //         }
    //     });
    // }
    //
    // public ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse> {
    //     return this.client.queryContractSmart(contractAddress, { owns_mixnode: { address } });
    // }
    //
    // public ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
    //     return this.client.queryContractSmart(contractAddress, { owns_gateway: { address } });
    // }
    //
    // public getContractSettingsParams(contractAddress: string): Promise<ContractSettingsParams> {
    //     return this.client.queryContractSmart(contractAddress, { contract_settings_params: {} });
    // }

}