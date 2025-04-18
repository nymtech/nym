"use client";
import { COSMOS_KIT_USE_CHAIN } from "@/config";
import { useChain } from "@cosmos-kit/react";
import { Stack, Typography, useTheme } from "@mui/material";
import React from "react";
import { Token } from "../../components/icons/Token";
import { TokenDark } from "../../components/icons/TokenDark";
import useGetWalletBalance from "../../hooks/useGetWalletBalance";

export const WalletBalance = () => {
  const { isWalletConnected } = useChain(COSMOS_KIT_USE_CHAIN);
  const { formattedBalance, isLoading, isError, refetch } =
    useGetWalletBalance();
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  if (isLoading) {
    return (
      <Stack direction="row" alignItems="center" gap={1}>
        {isDarkMode ? <TokenDark /> : <Token />}
        <Typography variant="h5" fontWeight={400}>
          Loading...
        </Typography>
      </Stack>
    );
  }
  if (isError) {
    if (isWalletConnected) {
      refetch();
    }
    return;
  }

  return (
    <Stack direction="row" alignItems="center" gap={1}>
      {isDarkMode ? <TokenDark /> : <Token />}
      <Typography variant="h5" fontWeight={400}>
        {formattedBalance} NYM
      </Typography>
    </Stack>
  );
};
