"use client";
import { Stack, Typography, useTheme } from "@mui/material";
import React from "react";
import { Elips } from "../../components/icons/Elips";
import { ElipseDark } from "../../components/icons/ElipseDark";

export const trimAddress = (address = "", trimBy = 6) =>
  `${address.slice(0, trimBy)}...${address.slice(-trimBy)}`;

export const WalletAddress = ({ address }: { address?: string }) => {
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  if (!address) {
    return null;
  }

  const displayAddress = trimAddress(address, 7);

  return (
    <Stack direction="row" alignItems="center" gap={0.5}>
      {isDarkMode ? <ElipseDark /> : <Elips />}
      <Typography variant="h5" fontWeight={400}>
        {displayAddress}
      </Typography>
    </Stack>
  );
};
