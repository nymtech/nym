// timer actions

import ValidatorClient, { Coin } from "../../src";
import { config } from "./config";

// Store current time as `start`
export const now = (eventName = null) => {
    if (eventName) {
      console.log(`Started ${eventName}..`);
    }
    return new Date().getTime();
  };
  
  //takes arg of start time
  export const elapsed = (beginning: number, log = false) => {
    const duration = new Date().getTime() - beginning;
    if (log) {
      console.log(`${duration / 1000}s`);
    }
    return duration;
  };
  
  export const profitPercentage = (): number => {
    return Math.floor(Math.random() * 100) + 1;
  };
  
  
  export const buildCoin = (amount: string, denomination: string): Coin => {
    return {
      denom: `u${denomination}`,
      amount: amount,
    };
  };
  
  export const buildWallet = async (): Promise<string> => {
      let mnemonic = ValidatorClient.randomMnemonic();
     
      const randomAddress = await ValidatorClient.buildWallet(
        mnemonic,
        config.NETWORK_BECH
      );
    let accountdetails = await randomAddress.getAccounts();
    let nymWallet = accountdetails[0].address;
    return nymWallet;
  };
  