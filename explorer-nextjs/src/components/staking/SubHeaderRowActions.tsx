"use client";

import { useNymClient } from "@/hooks/useNymClient";
import { Button, Stack } from "@mui/material";

const SubHeaderRowActions = () => {
  const { address } = useNymClient();

  if (!address) {
    return null;
  }
  return (
    <Stack direction="row" spacing={3} justifyContent={"end"}>
      <Button variant="outlined">Redeem all rewards</Button>
      <Button variant="contained">Stake NYM</Button>
    </Stack>
  );
};

export default SubHeaderRowActions;
