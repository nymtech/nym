import Header from "@/components/nav/Header";
import Stack from "@mui/material/Stack";
import type React from "react";

export default function NestedLayoutWithHeader({
  children,
  header,
}: { children?: React.ReactNode; header?: React.ReactNode }) {
  return (
    <Stack
      spacing={2}
      sx={{
        alignItems: "center",
        mx: 3,
        pb: 5,
        mt: { xs: 8, md: 0 },
      }}
    >
      <Header title={header} />
      {children}
    </Stack>
  );
}
