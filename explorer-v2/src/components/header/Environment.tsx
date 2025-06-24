"use client";
import React from "react";
import { Typography, Button, Link as MuiLink } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import { useEnvironment } from "../../providers/EnvironmentProvider";
import { useRouter, usePathname } from "next/navigation";
import { getBasePathByEnv } from "../../../envs/config";

export const Environment: React.FC = () => {
  const theme = useTheme();
  const { environment, setEnvironment } = useEnvironment();
  const router = useRouter();
  const pathname = usePathname();

  const explorerName = environment
    ? `${environment} Explorer`
    : "Mainnet Explorer";

  const switchNetworkText =
    environment === "mainnet" ? "Switch to Testnet" : "Switch to Mainnet";

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
    <Typography
      variant="h6"
      noWrap
      sx={{
        color: theme.palette.text.primary,
        fontSize: "18px",
        fontWeight: 600,
      }}
    >
      <MuiLink
        href={getBasePathByEnv(environment || "mainnet")}
        underline="none"
        color="inherit"
        textTransform="capitalize"
      >
        {explorerName}
      </MuiLink>
      <Button
        size="small"
        variant="outlined"
        color="inherit"
        onClick={handleSwitchEnvironment}
        sx={{
          borderRadius: 2,
          textTransform: "none",
          width: 150,
          ml: 4,
          fontSize: 14,
          fontWeight: 600,
        }}
      >
        {switchNetworkText}
      </Button>
    </Typography>
  );
};
