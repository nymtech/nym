"use client";

import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { GasPrice } from "@cosmjs/stargate";
import { useChain } from "@cosmos-kit/react";
import { Button, Stack } from "@mui/material";
import type { Delegation } from "@nymproject/contract-clients/Mixnet.types";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useState } from "react";
import { fetchTotalStakerRewards } from "../../app/api";
import type { NodeRewardDetails } from "../../app/api/types";
import { COSMOS_KIT_USE_CHAIN, NYM_MIXNET_CONTRACT } from "../../config";
import { useNymClient } from "../../hooks/useNymClient";
import Loading from "../loading";
import InfoModal, { type InfoModalProps } from "../modal/InfoModal";
import RedeemRewardsModal from "../redeemRewards/RedeemRewardsModal";

// Fetch delegations
const fetchDelegations = async (
  address: string,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  nymClient: any,
): Promise<Delegation[]> => {
  const data = await nymClient.getDelegatorDelegations({ delegator: address });
  return data.delegations;
};

const SubHeaderRowActions = () => {
  const [openRedeemRewardsModal, setOpenRedeemRewardsModal] =
    useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [infoModalProps, setInfoModalProps] = useState<InfoModalProps>({
    open: false,
  });

  const { address, nymClient } = useNymClient();
  const { getOfflineSigner } = useChain(COSMOS_KIT_USE_CHAIN);

  const queryClient = useQueryClient();

  // Fetch delegations using React Query
  const {
    data: delegations = [],
    isLoading: isDelegationsLoading,
    isError: isDelegationsError,
  } = useQuery({
    queryKey: ["delegations", address],
    queryFn: () => fetchDelegations(address || "", nymClient),
    enabled: !!address && !!nymClient, // Only fetch if address and nymClient are available
  });

  // Fetch total rewards using React Query
  const {
    data: totalStakerRewards = 0,
    isLoading: isRewardsLoading,
    isError: isRewardsError,
    refetch,
  } = useQuery({
    queryKey: ["totalStakerRewards", address],
    queryFn: () => fetchTotalStakerRewards(address || ""),
    enabled: !!address, // Only fetch if address is available
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });

  const handleRefetch = useCallback(async () => {
    refetch();
    queryClient.invalidateQueries(); // This will refetch ALL active queries
  }, [queryClient, refetch]);

  const handleRedeemRewards = useCallback(async () => {
    setIsLoading(true);
    setOpenRedeemRewardsModal(false);

    try {
      if (!nymClient || !address || !delegations.length) {
        throw new Error("Wallet, client, or delegations not available.");
      }

      const signer = await getOfflineSigner();
      const gasPrice = GasPrice.fromString("0.025unym");

      const client = await SigningCosmWasmClient.connectWithSigner(
        "https://rpc.nymtech.net/",
        signer,
        { gasPrice },
      );

      const messages = delegations.map((delegation: NodeRewardDetails) => ({
        contractAddress: NYM_MIXNET_CONTRACT,
        funds: [],
        msg: {
          withdraw_delegator_reward: {
            node_id: delegation.node_id,
          },
        },
      }));

      const result = await client.executeMultiple(
        address,
        messages,
        "auto",
        "Redeeming all rewards",
      );
      // Success state
      setIsLoading(false);
      setInfoModalProps({
        open: true,
        title: "Success",
        message: "All rewards have been redeemed successfully!",
        tx: result?.transactionHash,

        onClose: async () => {
          await handleRefetch();
          setInfoModalProps({ open: false });
        },
      });
    } catch (error) {
      console.error("Error redeeming rewards:", error);
      setInfoModalProps({
        open: true,
        title: "Error",
        message:
          error instanceof Error ? error.message : "Failed to redeem rewards.",
        onClose: () => setInfoModalProps({ open: false }),
      });
    } finally {
      setIsLoading(false);
    }
  }, [address, nymClient, delegations, handleRefetch, getOfflineSigner]);

  const handleRedeemRewardsButtonClick = () => {
    setOpenRedeemRewardsModal(true);
  };

  if (!address || !nymClient) {
    return null;
  }

  if (isDelegationsLoading || isRewardsLoading) {
    return <Loading />;
  }

  if (isDelegationsError || isRewardsError) {
    return (
      <Stack direction="row" spacing={3} justifyContent={"end"}>
        <Button variant="contained" disabled>
          Error loading data
        </Button>
      </Stack>
    );
  }

  return (
    <Stack direction="row" spacing={3} justifyContent={"end"}>
      <Button
        variant="contained"
        onClick={handleRedeemRewardsButtonClick}
        disabled={totalStakerRewards === 0}
      >
        Redeem NYM
      </Button>
      {isLoading && <Loading />}
      {openRedeemRewardsModal && (
        <RedeemRewardsModal
          onRedeem={handleRedeemRewards}
          onClose={() => setOpenRedeemRewardsModal(false)}
          totalRewardsAmount={totalStakerRewards}
        />
      )}
      <InfoModal {...infoModalProps} />
    </Stack>
  );
};

export default SubHeaderRowActions;
