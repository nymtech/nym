"use client";
import { Search } from "@mui/icons-material";
import { Button, Stack } from "@mui/material";
import { useRouter } from "next/navigation";
import Input from "../input/Input";

const NodeAndAddressSearch = () => {
  const router = useRouter();
  return (
    <Stack spacing={4} direction="row">
      <Input placeholder="Node ID / Nym Address" fullWidth />
      <Button
        variant="contained"
        endIcon={<Search />}
        size="large"
        onClick={() => router.push("/nym-node/123")}
      >
        Search
      </Button>
    </Stack>
  );
};

export default NodeAndAddressSearch;
