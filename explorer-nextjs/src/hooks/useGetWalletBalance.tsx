import { COSMOS_KIT_USE_CHAIN } from "@/app/api/urls";
import { unymToNym } from "@/utils/currency";
import { useChain } from "@cosmos-kit/react";
import { useCallback, useEffect, useState } from "react";

const useGetWalletBalance = () => {
  const [balance, setBalance] = useState<string>("-");
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
      return formattedBalance;
    },
    [getCosmWasmClient],
  );

  useEffect(() => {
    if (!address) {
      return;
    }

    getNYMBalance(address).then((balance) => {
      setBalance(balance || "-");
    });
  }, [address, getNYMBalance]);

  return { balance };
};

export default useGetWalletBalance;
