import { useCallback, useContext, useState } from "react";
import { Coin, printableCoin } from "@nymproject/nym-validator-client";
import { ValidatorClientContext } from "../contexts/ValidatorClient";
import { basicRawCoinValueValidation } from "../common/helpers";

export const useGetBalance = () => {
  const { client } = useContext(ValidatorClientContext);
  const [isLoading, setIsLoading] = useState(false);
  const [balanceCheckError, setBalanceCheckError] = useState(null);
  const [accountBalance, setAccountBalance] = useState<Coin>();

  console.log(basicRawCoinValueValidation(accountBalance?.amount));

  const getBalance = useCallback(async () => {
    if (client) {
      setIsLoading(true);
      console.log(`using the context client, our address is ${client.address}`);

      try {
        const value = await client.getBalance(client.address);
        setAccountBalance(value);
        setIsLoading(false);
      } catch (e) {
        setBalanceCheckError(e);
      }
    }
  }, []);

  return {
    balanceCheckError,
    isBalanceLoading: isLoading,
    accountBalance,
    printedBalance: printableCoin(accountBalance),
    getBalance,
  };
};
