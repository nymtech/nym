"use client";
import React from "react";
import { Typography, Button, Link as MuiLink } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import { useEnvironment } from "../../providers/EnvironmentProvider";

export const Environment: React.FC = () => {
  const theme = useTheme();
  const { environment, setEnvironment } = useEnvironment();


  const explorerName = environment
    ? `${environment} Explorer`
    : "Mainnet Explorer";

  const switchNetworkText =
    environment === "mainnet" ? "Switch to Testnet" : "Switch to Mainnet";

  const switchNetworkLink = environment === "mainnet" ? "/" : "/";

  const handleSwitchEnvironment = () => {
    setEnvironment(environment === "mainnet" ? "sandbox" : "mainnet");
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
        href="/"
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
        href={switchNetworkLink}
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
