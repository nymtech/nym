"use client";
import React from "react";
import { Button } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import { useEnvironment } from "../../providers/EnvironmentProvider";
import { useRouter, usePathname } from "next/navigation";
import { getBasePathByEnv } from "../../../envs/config";
import { colours } from "@/theme/colours";

export const EnvironmentSwitcher: React.FC = () => {
  const theme = useTheme();
  const { environment, setEnvironment } = useEnvironment();
  const router = useRouter();
  const pathname = usePathname();

  const switchNetworkText =
    environment === "mainnet" ? "Switch to Sandbox" : "Switch to Mainnet";

  const getCurrentInternalPath = () => {
    // Remove the base path from the current pathname to get the internal path
    return pathname.replace(/^\/(explorer|sandbox-explorer)/, "") || "/";
  };

  const handleSwitchEnvironment = () => {
    const newEnvironment = environment === "mainnet" ? "sandbox" : "mainnet";
    setEnvironment(newEnvironment);

    // Get the current internal path and build the new path
    const currentInternalPath = getCurrentInternalPath();
    const newBasePath = getBasePathByEnv(newEnvironment);
    const newPath =
      currentInternalPath === "/"
        ? newBasePath
        : `${newBasePath}${currentInternalPath}`;
    router.push(newPath);
  };

  return (
    <Button
      variant="outlined"
      color="inherit"
      onClick={handleSwitchEnvironment}
      sx={{
        borderRadius: 2,
        px: 2,
        py: 1,
        color:
          theme.palette.mode === "light"
            ? `${theme.palette.common.black} !important`
            : `${theme.palette.common.white} !important`,
        borderColor:
          theme.palette.mode === "light"
            ? theme.palette.common.black
            : theme.palette.common.white,
        borderStyle: environment === "sandbox" ? "solid" : "dashed",
        backgroundColor:
          environment === "sandbox" && theme.palette.mode === "dark"
            ? colours.pine[800]
            : environment === "sandbox" && theme.palette.mode === "light"
              ? colours.pine[300]
              : "transparent",
        fontWeight: 500,
        fontSize: 14,
      }}
    >
      {switchNetworkText}
    </Button>
  );
};
