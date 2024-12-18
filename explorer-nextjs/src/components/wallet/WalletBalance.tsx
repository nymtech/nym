import { COSMOS_KIT_USE_CHAIN } from "@/app/api/urls";
import { Token } from "@/components/icons/Token";
import { unymToNym } from "@/utils/currency";
import { useChain } from "@cosmos-kit/react";
import { Stack, Typography } from "@mui/material";
import React, { useCallback, useEffect, useState } from "react";

export const WalletBalance = ({ address }: { address: string }) => {
  const [balance, setBalance] = useState<string>("-");
  const { getCosmWasmClient } = useChain(COSMOS_KIT_USE_CHAIN);

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
    getNYMBalance(address).then((balance) => {
      if (balance) {
        setBalance(balance);
      }
    });
  }, [address, getNYMBalance]);

  if (!address) {
    return null;
  }

  return (
    <Stack direction="row" alignItems="center" gap={1}>
      <Token />
      <Typography variant="h5" fontWeight={400}>
        {balance} NYM
      </Typography>
    </Stack>
  );
};
