import Copyright from "@/components/Copyright";
import AppNavbar from "@/components/nav/AppNavbar";
import SideMenu from "@/components/nav/SideMenu";
import Box from "@mui/material/Box";
import { alpha } from "@mui/material/styles";
import type React from "react";

export default function LayoutWithNav({
  children,
}: { children?: React.ReactNode }) {
  return (
    <Box sx={{ display: "flex" }}>
      <SideMenu />
      <AppNavbar />
      {/* Main content */}
      <Box
        component="main"
        sx={(theme) => ({
          flexGrow: 1,
          backgroundColor: alpha(theme.palette.background.default, 1),
          overflow: "auto",
        })}
      >
        {children}
        <Copyright />
      </Box>
    </Box>
  );
}
