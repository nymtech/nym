import { Stack, Typography } from "@mui/material";
import React from "react";
import { Elips } from "../../components/icons/Elips";

export const trimAddress = (address = "", trimBy = 6) =>
  `${address.slice(0, trimBy)}...${address.slice(-trimBy)}`;

export const WalletAddress = ({ address }: { address?: string }) => {
  if (!address) {
    return null;
  }

  const displayAddress = trimAddress(address, 7);

  return (
    <Stack direction="row" alignItems="center" gap={0.5}>
      <Elips />
      <Typography variant="h5" fontWeight={400}>
        {displayAddress}
      </Typography>
    </Stack>
  );
};
