"use client";
import type { IObservatoryNode } from "@/app/api/types";
import { COSMOS_KIT_USE_CHAIN } from "@/config";
import { useNymClient } from "@/hooks/useNymClient";
import { useChain } from "@cosmos-kit/react";
import { Box, Button, Stack, Typography } from "@mui/material";
import { useCallback, useState } from "react";
import { RandomAvatar } from "react-random-avatars";
import ExplorerCard from "../cards/ExplorerCard";
import CountryFlag from "../countryFlag/CountryFlag";
import Loading from "../loading";
import InfoModal, { type InfoModalProps } from "../modal/InfoModal";
import StakeModal from "../staking/StakeModal";
import { fee } from "../staking/schemas";
import ConnectWallet from "../wallet/ConnectWallet";

interface INodeProfileCardProps {
  nodeInfo: IObservatoryNode;
}

export const NodeProfileCard = (props: INodeProfileCardProps) => {
  const { nodeInfo } = props;
  const { isWalletConnected } = useChain(COSMOS_KIT_USE_CHAIN);
  const { nymClient } = useNymClient();
  const [infoModalProps, setInfoModalProps] = useState<InfoModalProps>({
    open: false,
  });
  const [isLoading, setIsLoading] = useState(false);
  const [selectedNodeForStaking, setSelectedNodeForStaking] = useState<{
    nodeId: number;
    identityKey: string;
  }>();

  const handleStakeOnNode = async ({
    nodeId,
    amount,
  }: {
    nodeId: number;
    amount: string;
  }) => {
    const amountToDelegate = (Number(amount) * 1_000_000).toString();
    const uNymFunds = [{ amount: amountToDelegate, denom: "unym" }];

    setIsLoading(true);
    setSelectedNodeForStaking(undefined);
    try {
      const tx = await nymClient?.delegate(
        { nodeId },
        fee,
        "Delegation from Nym Explorer V2",
        uNymFunds,
      );
      console.log({ tx });
      setSelectedNodeForStaking(undefined);
      setInfoModalProps({
        open: true,
        title: "Success",
        message: "This operation can take up to one hour to process",
        tx: tx?.transactionHash,

        onClose: () => setInfoModalProps({ open: false }),
      });
    } catch (e) {
      const errorMessage =
        e instanceof Error ? e.message : "An error occurred while staking";
      setInfoModalProps({
        open: true,
        title: "Error",
        message: errorMessage,
        onClose: () => {
          setInfoModalProps({ open: false });
        },
      });
    }
    setIsLoading(false);
  };

  const handleOnSelectStake = useCallback(() => {
    if (!isWalletConnected) {
      setInfoModalProps({
        open: true,
        title: "Connect Wallet",
        message: "Connect your wallet to stake",
        Action: (
          <ConnectWallet
            fullWidth
            onClick={() =>
              setInfoModalProps({
                open: false,
              })
            }
          />
        ),
        onClose: () => setInfoModalProps({ open: false }),
      });
      return;
    }
    setSelectedNodeForStaking({
      nodeId: nodeInfo.node_id,
      identityKey: nodeInfo.identity_key,
    });
  }, [isWalletConnected, nodeInfo]);

  return (
    <ExplorerCard label="Nym Node" sx={{ height: "100%" }}>
      <Stack gap={1}>
        <Box display={"flex"} justifyContent={"flex-start"}>
          <RandomAvatar name={nodeInfo.identity_key} size={80} square />
        </Box>
        <Typography
          variant="h3"
          mt={3}
          mb={1}
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          {nodeInfo?.self_description.moniker || "Moniker"}
        </Typography>
        {nodeInfo.description.auxiliary_details.location && (
          <CountryFlag
            countryCode={nodeInfo.description.auxiliary_details.location}
            countryName={nodeInfo.description.auxiliary_details.location}
          />
        )}
        {nodeInfo && (
          <Typography variant="body4" sx={{ color: "pine.950" }} mt={2}>
            {nodeInfo.self_description.details}
          </Typography>
        )}
        <Box mt={3}>
          <Button
            variant="contained"
            size="small"
            onClick={handleOnSelectStake}
          >
            Stake on node
          </Button>
        </Box>
      </Stack>
      {isLoading && <Loading />}
      <StakeModal
        nodeId={selectedNodeForStaking?.nodeId}
        identityKey={selectedNodeForStaking?.identityKey}
        onStake={handleStakeOnNode}
        onClose={() => setSelectedNodeForStaking(undefined)}
      />
      <InfoModal {...infoModalProps} />
    </ExplorerCard>
  );
};
