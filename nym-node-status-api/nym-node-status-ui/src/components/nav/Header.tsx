"use client";

import ColorModeIconDropdown from "@/theme/ColorModeIconDropdown";
import Stack from "@mui/material/Stack";
import NavbarBreadcrumbs from "./NavbarBreadcrumbs";

import type React from "react";
import Search from "./Search";

const showSearch = false;

export default function Header({ title }: { title?: React.ReactNode }) {
  return (
    <Stack
      direction="row"
      sx={{
        display: { xs: "none", md: "flex" },
        width: "100%",
        alignItems: { xs: "flex-start", md: "center" },
        justifyContent: "space-between",
        maxWidth: { sm: "100%", md: "1700px" },
        pt: 1.5,
      }}
      spacing={2}
    >
      <NavbarBreadcrumbs title={title} />
      <Stack direction="row" sx={{ gap: 1 }}>
        {showSearch && <Search />}
        <ColorModeIconDropdown />
      </Stack>
    </Stack>
  );
}
