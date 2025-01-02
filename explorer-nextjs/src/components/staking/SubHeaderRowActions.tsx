"use client";

import { useNymClient } from "@/hooks/useNymClient";
import { Button, Stack } from "@mui/material";
import { Link } from "../muiLink";

const SubHeaderRowActions = () => {
  const { address } = useNymClient();

  if (!address) {
    return null;
  }

  return (
    <Stack direction="row" spacing={3} justifyContent={"end"}>
      <Button variant="outlined">Redeem all rewards</Button>
      <Link href="/explorer">
        <Button variant="contained">Stake NYM</Button>
      </Link>
    </Stack>
  );
};

export default SubHeaderRowActions;
