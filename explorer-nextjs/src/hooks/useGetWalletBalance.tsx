import { COSMOS_KIT_USE_CHAIN } from "@/config";
import { unymToNym } from "@/utils/currency";
import { useChain } from "@cosmos-kit/react";
import { useCallback, useEffect, useState } from "react";

const useGetWalletBalance = () => {
  const [balance, setBalance] = useState<string>("0");
  const [formattedBalance, setFormattedBalance] = useState<string>("-");
  const { getCosmWasmClient, address } = useChain(COSMOS_KIT_USE_CHAIN);

  const getNYMBalance = useCallback(
    async (address: string) => {
      const account = await getCosmWasmClient();
      const uNYMBalance = await account.getBalance(address, "unym");
      const NYMBalance = unymToNym(uNYMBalance.amount);
      if (!NYMBalance) {
        return undefined;
      }
      const formattedBalance = Intl.NumberFormat().format(+NYMBalance);
      return {
        NYMBalance,
        formattedBalance,
      };
    },
    [getCosmWasmClient],
  );

  useEffect(() => {
    if (!address) {
      return;
    }

    getNYMBalance(address)
      .then((balance) => {
        setFormattedBalance(balance?.formattedBalance || "-");
        setBalance(balance?.NYMBalance || "0");
      })
      .catch((e) => {
        console.error("Failed to get balance", e);
      });
  }, [address, getNYMBalance]);

  return { balance, formattedBalance };
};

export default useGetWalletBalance;
