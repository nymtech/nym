"use client";

import type { IObservatoryNode } from "@/app/api/types";
import { useChain } from "@cosmos-kit/react";
import {
  Box,
  Button,
  Skeleton,
  Stack,
  Typography,
  useTheme,
} from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import DOMPurify from "isomorphic-dompurify";
import { useCallback, useState } from "react";
import { RandomAvatar } from "react-random-avatars";
import { fetchObservatoryNodes } from "../../app/api";
import { COSMOS_KIT_USE_CHAIN } from "../../config";
import { useNymClient } from "../../hooks/useNymClient";
import ExplorerCard from "../cards/ExplorerCard";
import CountryFlag from "../countryFlag/CountryFlag";
import { Favorite } from "../favorite/Favorite";
import Loading from "../loading";
import InfoModal, { type InfoModalProps } from "../modal/InfoModal";
import StakeModal from "../staking/StakeModal";
import { fee } from "../staking/schemas";
import ConnectWallet from "../wallet/ConnectWallet";

type Props = {
  paramId: string;
};

export const NodeProfileCard = ({ paramId }: Props) => {
  let nodeInfo: IObservatoryNode | undefined;
  const theme = useTheme();

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

  // Fetch node info
  const {
    data: nymNodes,
    isLoading: isLoadingNymNodes,
    isError,
  } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: fetchObservatoryNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // get node info based on wether it's dentity_key or node_id

  if (paramId.length > 10) {
    nodeInfo = nymNodes?.find((node) => node.identity_key === paramId);
  } else {
    nodeInfo = nymNodes?.find((node) => node.node_id === Number(paramId));
  }

  const handleOnSelectStake = useCallback(() => {
    if (!nodeInfo) return;
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
    if (nodeInfo) {
      setSelectedNodeForStaking({
        nodeId: nodeInfo.node_id,
        identityKey: nodeInfo.identity_key,
      });
    }
  }, [isWalletConnected, nodeInfo]);

  if (isLoadingNymNodes) {
    return (
      <ExplorerCard label="Nym Node" sx={{ height: "100%" }}>
        <Skeleton variant="rectangular" height={80} width={80} />
        <Skeleton variant="text" />
        <Skeleton variant="text" height={200} />
      </ExplorerCard>
    );
  }
  if (isError || !nymNodes) {
    return (
      <ExplorerCard label="Nym Node" sx={{ height: "100%" }}>
        <Typography
          variant="h3"
          sx={{
            color: theme.palette.mode === "dark" ? "base.white" : "pine.950",
          }}
        >
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }

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

  if (!nodeInfo) return null;

  const cleanMoniker = DOMPurify.sanitize(
    nodeInfo?.self_description.moniker,
  ).replace(/&amp;/g, "&");

  const cleanDescription = DOMPurify.sanitize(
    nodeInfo?.self_description.details,
  ).replace(/&amp;/g, "&");

  // get full country name
  const countryName = (countryCode: string) => {
    const regionNames = new Intl.DisplayNames(["en"], { type: "region" });

    return <span>{regionNames.of(countryCode)}</span>;
  };

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
          sx={{
            color: theme.palette.mode === "dark" ? "base.white" : "pine.950",
            wordWrap: "break-word",
            maxWidth: "95%",
          }}
        >
          {cleanMoniker || "Moniker"}
        </Typography>
        {nodeInfo.description.auxiliary_details.location && (
          <Box display={"flex"} gap={1}>
            <Typography
              variant="h6"
              sx={{
                color: theme.palette.mode === "dark" ? "base.white" : "inherit",
              }}
            >
              Location:
            </Typography>

            <Box>
              <CountryFlag
                countryCode={nodeInfo.description.auxiliary_details.location}
                countryName={countryName(
                  nodeInfo.description.auxiliary_details.location,
                )}
              />
            </Box>
          </Box>
        )}
        {nodeInfo && (
          <Typography
            variant="body4"
            sx={{
              color: theme.palette.mode === "dark" ? "base.white" : "pine.950",
            }}
            mt={2}
          >
            {cleanDescription}
          </Typography>
        )}
        <Box mt={3} display={"flex"} gap={1}>
          <Button
            variant="contained"
            size="small"
            onClick={handleOnSelectStake}
          >
            Stake on node
          </Button>
          <Favorite address={nodeInfo.bonding_address} />
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
