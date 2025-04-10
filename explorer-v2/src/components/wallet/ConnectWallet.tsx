"use client";

import { useChain } from "@cosmos-kit/react";
import {
  Box,
  Button,
  type ButtonProps,
  IconButton,
  Typography,
  useTheme,
} from "@mui/material";
import { COSMOS_KIT_USE_CHAIN } from "../../config";
import Cross from "../icons/Cross";
import CrossDark from "../icons/CrossDark";
import { WalletAddress } from "./WalletAddress";
import { WalletBalance } from "./WalletBalance";

interface ButtonPropsWithOnClick extends ButtonProps {
  hideAddressAndBalance?: boolean;
  onClick?: () => void;
}

const ConnectWallet = ({ ...buttonProps }: ButtonPropsWithOnClick) => {
  const { connect, disconnect, address, isWalletConnected } =
    useChain(COSMOS_KIT_USE_CHAIN);
  const theme = useTheme();

  const handleConnectWallet = async () => {
    buttonProps.onClick?.();
    await connect();
  };

  const handleDisconnectWallet = async () => {
    await disconnect();
  };

  if (isWalletConnected && !buttonProps.hideAddressAndBalance) {
    return (
      <Box
        display={"flex"}
        flexDirection={{ xs: "column", sm: "row" }}
        alignItems={"center"}
        gap={{ xs: 2, sm: 3 }}
      >
        <WalletBalance />
        <WalletAddress address={address} />
        <Box>
          <IconButton
            size="small"
            onClick={async () => {
              await handleDisconnectWallet();
            }}
          >
            <Box height={24} display={"flex"} alignItems={"center"}>
              <Typography
                variant="h5"
                fontWeight={400}
                mr={1}
                display={{ sm: "none" }}
                sx={{ color: "unset" }}
              >
                Disconnect
              </Typography>
              {theme.palette.mode === "dark" ? <CrossDark /> : <Cross />}
            </Box>
          </IconButton>
        </Box>
      </Box>
    );
  }

  return (
    <Button
      fullWidth={buttonProps.fullWidth}
      variant="contained"
      size={buttonProps.size}
      onClick={handleConnectWallet}
    >
      Connect Wallet
    </Button>
  );
};

export default ConnectWallet;
