import { useContext, useState } from "react";
import { printableCoin } from "@nymproject/nym-validator-client";
import { ValidatorClientContext } from "../contexts/ValidatorClient";

export const useGetBalance = () => {
  const { client } = useContext(ValidatorClientContext);
  const [isLoading, setIsLoading] = useState(false);
  const [balanceCheckError, setBalanceCheckError] = useState(null);
  const [accountBalance, setAccountBalance] = useState("");

  const getBalance = async () => {
    if (client) {
      setIsLoading(true);
      console.log(`using the context client, our address is ${client.address}`);

      try {
        const value = await client.getBalance(client.address);
        setAccountBalance(printableCoin(value));
        setIsLoading(false);
      } catch (e) {
        setBalanceCheckError(e);
      }
    }
  };

  return {
    balanceCheckError,
    isBalanceLoading: isLoading,
    accountBalance,
    getBalance,
  };
};
