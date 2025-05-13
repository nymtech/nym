"use client";

import GatewaysTable from "@/app/dvpn/GatewaysTable";
import NestedLayoutWithHeader from "@/layouts/NestedLayoutWithHeader";
import Box from "@mui/material/Box";

export default function Page() {
  return (
    <NestedLayoutWithHeader header="dVPN Gateways">
      <Box width="100%">
        <GatewaysTable />
      </Box>
    </NestedLayoutWithHeader>
  );
}
