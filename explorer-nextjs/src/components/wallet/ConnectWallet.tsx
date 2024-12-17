"use client";

import { COSMOS_KIT_USE_CHAIN } from "@/app/api/urls";
import { useChain } from "@cosmos-kit/react";
import CloseIcon from "@mui/icons-material/Close";
import { Button, IconButton, Stack } from "@mui/material";
import Cross from "../icons/Cross";
import { WalletAddress } from "./WalletAddress";
import { WalletBalance } from "./WalletBalance";

const ConnectWallet = () => {
  const { connect, disconnect, address, isWalletConnected } =
    useChain(COSMOS_KIT_USE_CHAIN);

  const handleConnectWallet = async () => {
    await connect();
  };

  const handleDisconnectWallet = async () => {
    await disconnect();
  };

  if (isWalletConnected) {
    return (
      <Stack direction="row" spacing={1}>
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
    <Button variant="contained" size="small" onClick={handleConnectWallet}>
      Connect Wallet
    </Button>
  );
};

export default ConnectWallet;
