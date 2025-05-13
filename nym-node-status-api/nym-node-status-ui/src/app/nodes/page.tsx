"use client";

import NodesTable from "@/app/nodes/NodesTable";
import NestedLayoutWithHeader from "@/layouts/NestedLayoutWithHeader";
import Box from "@mui/material/Box";

export default function Page() {
  return (
    <NestedLayoutWithHeader header="Nym Network Nodes">
      <Box width="100%">
        <NodesTable />
      </Box>
    </NestedLayoutWithHeader>
  );
}
