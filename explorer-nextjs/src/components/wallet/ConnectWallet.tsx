"use client";

import { COSMOS_KIT_USE_CHAIN } from "@/config";
import { useChain } from "@cosmos-kit/react";
import { Button, type ButtonProps, IconButton, Stack } from "@mui/material";
import Cross from "../icons/Cross";
import { WalletAddress } from "./WalletAddress";
import { WalletBalance } from "./WalletBalance";

interface ButtonPropsWithOnClick extends ButtonProps {
  hideAddressAndBalance?: boolean;
  onClick?: () => void;
}

const ConnectWallet = ({ ...buttonProps }: ButtonPropsWithOnClick) => {
  const { connect, disconnect, address, isWalletConnected } =
    useChain(COSMOS_KIT_USE_CHAIN);

  const handleConnectWallet = async () => {
    buttonProps.onClick?.();
    await connect();
  };

  const handleDisconnectWallet = async () => {
    await disconnect();
  };

  if (isWalletConnected && !buttonProps.hideAddressAndBalance) {
    return (
      <Stack direction="row" spacing={3}>
        <WalletBalance />
        <WalletAddress address={address} />
        <IconButton
          size="small"
          onClick={async () => {
            await handleDisconnectWallet();
          }}
        >
          <Cross />
        </IconButton>
      </Stack>
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
