import { useChain } from "@cosmos-kit/react";
import { useQuery } from "@tanstack/react-query";
import { COSMOS_KIT_USE_CHAIN } from "../config";
import { unymToNym } from "../utils/currency";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const fetchNYMBalance = async (address: string, getCosmWasmClient: any) => {
  if (!address) return { NYMBalance: "0", formattedBalance: "-" };

  const account = await getCosmWasmClient();
  const uNYMBalance = await account.getBalance(address, "unym");
  const NYMBalance = unymToNym(uNYMBalance.amount);
  if (!NYMBalance) return;
  const formattedBalance = Intl.NumberFormat().format(+NYMBalance);

  return { NYMBalance, formattedBalance };
};

const useGetWalletBalance = () => {
  const { getCosmWasmClient, address } = useChain(COSMOS_KIT_USE_CHAIN);

  const {
    data = { NYMBalance: "0", formattedBalance: "-" },
    isLoading,
    isError,
    refetch,
  } = useQuery({
    queryKey: ["nymBalance", address],
    queryFn: () => fetchNYMBalance(address || "", getCosmWasmClient),
    enabled: !!address, // Only fetch if address exists
  });

  return {
    balance: data.NYMBalance,
    formattedBalance: data.formattedBalance,
    isLoading,
    isError,
    refetch, // Expose refetch function if needed
  };
};

export default useGetWalletBalance;
