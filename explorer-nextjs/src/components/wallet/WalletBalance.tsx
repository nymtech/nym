import { Stack, Typography } from "@mui/material";
import React from "react";
import { Token } from "../../components/icons/Token";
import useGetWalletBalance from "../../hooks/useGetWalletBalance";

export const WalletBalance = () => {
  const { formattedBalance } = useGetWalletBalance();

  return (
    <Stack direction="row" alignItems="center" gap={1}>
      <Token />
      <Typography variant="h5" fontWeight={400}>
        {formattedBalance} NYM
      </Typography>
    </Stack>
  );
};
