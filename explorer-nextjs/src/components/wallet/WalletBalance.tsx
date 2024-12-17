import { Token } from "@/components/icons/Token";
import { Stack, Typography } from "@mui/material";
import React from "react";

export const WalletBalance = () => {
  return (
    <Stack direction="row" alignItems="center" gap={1}>
      <Token />
      <Typography variant="h5" fontWeight={400}>
        {"100"} NYM
      </Typography>
    </Stack>
  );
};
