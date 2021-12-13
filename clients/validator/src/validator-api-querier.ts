/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

import {GatewayBond, MixNodeBond} from "./types";
import axios from "axios";

export const VALIDATOR_API_VERSION = "/v1";
export const VALIDATOR_API_GATEWAYS_PATH = VALIDATOR_API_VERSION + "/gateways";
export const VALIDATOR_API_MIXNODES_PATH = VALIDATOR_API_VERSION + "/mixnodes";
export const VALIDATOR_API_ACTIVE_MIXNODES_PATH = VALIDATOR_API_VERSION + "/mixnodes/active";
export const VALIDATOR_API_REWARDED_MIXNODES_PATH = VALIDATOR_API_VERSION + "/mixnodes/rewarded";

export interface IValidatorApiQuery {
    getCachedMixnodes(): Promise<MixNodeBond[]>
    getCachedGateways(): Promise<GatewayBond[]>
    getActiveMixnodes(): Promise<MixNodeBond[]>
    getRewardedMixnodes(): Promise<MixNodeBond[]>
}

export default class ValidatorApiQuerier implements IValidatorApiQuery {
    validatorApiUrl: string;

    constructor(validatorApiUrl: string) {
        this.validatorApiUrl = validatorApiUrl
    }

    async getCachedMixnodes(): Promise<MixNodeBond[]> {
        const url = new URL(this.validatorApiUrl)
        url.pathname += VALIDATOR_API_MIXNODES_PATH

        const response = await axios.get(url.toString())
        if (response.status == 200) {
            return response.data;
        } else {
            throw new Error("None of the provided validator APIs seem to be alive")
        }
    }

    async getCachedGateways(): Promise<GatewayBond[]> {
        const url = new URL(this.validatorApiUrl)
        url.pathname += VALIDATOR_API_GATEWAYS_PATH

        const response = await axios.get(url.toString())
        if (response.status == 200) {
            return response.data;
        } else {
            throw new Error("None of the provided validator APIs seem to be alive")
        }
    }

    async getActiveMixnodes(): Promise<MixNodeBond[]> {
        const url = new URL(this.validatorApiUrl)
        url.pathname += VALIDATOR_API_ACTIVE_MIXNODES_PATH

        const response = await axios.get(url.toString())
        if (response.status == 200) {
            return response.data;
        } else {
            throw new Error("None of the provided validator APIs seem to be alive")
        }
    }

    async getRewardedMixnodes(): Promise<MixNodeBond[]> {
        const url = new URL(this.validatorApiUrl)
        url.pathname += VALIDATOR_API_REWARDED_MIXNODES_PATH

        const response = await axios.get(url.toString())
        if (response.status == 200) {
            return response.data;
        } else {
            throw new Error("None of the provided validator APIs seem to be alive")
        }
    }
}