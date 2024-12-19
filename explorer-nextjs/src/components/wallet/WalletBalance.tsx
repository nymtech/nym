import { Token } from "@/components/icons/Token";
import useGetWalletBalance from "@/hooks/useGetWalletBalance";
import { Stack, Typography } from "@mui/material";
import React from "react";

export const WalletBalance = () => {
  const { balance } = useGetWalletBalance();

  return (
    <Stack direction="row" alignItems="center" gap={1}>
      <Token />
      <Typography variant="h5" fontWeight={400}>
        {balance} NYM
      </Typography>
    </Stack>
  );
};
