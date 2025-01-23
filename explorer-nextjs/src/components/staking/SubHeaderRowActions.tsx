"use client";

import type { NodeRewardDetails, ObservatoryBalance } from "@/app/api/types";
import { DATA_OBSERVATORY_BALANCES_URL } from "@/app/api/urls";
import { COSMOS_KIT_USE_CHAIN, NYM_MIXNET_CONTRACT } from "@/config";
import { useNymClient } from "@/hooks/useNymClient";
import { useChain } from "@cosmos-kit/react";
import { Button, Stack } from "@mui/material";
import type { Delegation } from "@nymproject/contract-clients/Mixnet.types";
import { useQuery } from "@tanstack/react-query";
import { useCallback, useState } from "react";
import Loading from "../loading";
import InfoModal, { type InfoModalProps } from "../modal/InfoModal";
import RedeemRewardsModal from "../redeemRewards/RedeemRewardsModal";

const fee = { gas: "1000000", amount: [{ amount: "1000000", denom: "unym" }] };

// Fetch delegations
const fetchDelegations = async (
  address: string,
  nymClient: any,
): Promise<Delegation[]> => {
  const data = await nymClient.getDelegatorDelegations({ delegator: address });
  return data.delegations;
};

// Fetch total staker rewards
const fetchTotalRewards = async (address: string): Promise<number> => {
  const response = await fetch(`${DATA_OBSERVATORY_BALANCES_URL}/${address}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch balances");
  }

  const balances: ObservatoryBalance = await response.json();
  return Number(balances.rewards.staking_rewards.amount);
};

const SubHeaderRowActions = () => {
  const [openRedeemRewardsModal, setOpenRedeemRewardsModal] =
    useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [infoModalProps, setInfoModalProps] = useState<InfoModalProps>({
    open: false,
  });

  const { address, nymClient } = useNymClient();
  const { getSigningCosmWasmClient } = useChain(COSMOS_KIT_USE_CHAIN);

  // Fetch delegations using React Query
  const {
    data: delegations = [],
    isLoading: isDelegationsLoading,
    isError: isDelegationsError,
  } = useQuery({
    queryKey: ["delegations", address],
    queryFn: () => fetchDelegations(address || "", nymClient),
    enabled: !!address && !!nymClient, // Only fetch if address and nymClient are available
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000,
  });

  // Fetch total rewards using React Query
  const {
    data: totalStakerRewards = 0,
    isLoading: isRewardsLoading,
    isError: isRewardsError,
  } = useQuery({
    queryKey: ["totalRewards", address],
    queryFn: () => fetchTotalRewards(address || ""),
    enabled: !!address, // Only fetch if address is available
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000,
  });

  const handleRedeemRewards = useCallback(async () => {
    setIsLoading(true);
    setOpenRedeemRewardsModal(false);

    try {
      if (!nymClient || !address || !delegations.length) {
        throw new Error("Wallet, client, or delegations not available.");
      }

      console.log("delegations :>> ", delegations);

      const messages = delegations.map((delegation: NodeRewardDetails) => ({
        contractAddress: NYM_MIXNET_CONTRACT,
        funds: [],
        msg: {
          withdraw_delegator_reward: {
            node_id: delegation.node_id,
          },
        },
      }));

      const cosmWasmSigningClient = await getSigningCosmWasmClient();

      const result = await cosmWasmSigningClient.executeMultiple(
        address,
        messages,
        fee,
        "Redeeming all rewards",
      );
      console.log("Rewards redeemed successfully:", result);

      setInfoModalProps({
        open: true,
        title: "Success",
        message: "All rewards have been redeemed successfully!",
        onClose: () => setInfoModalProps({ open: false }),
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
  }, [address, nymClient, delegations, getSigningCosmWasmClient]);

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
      <Button variant="contained" onClick={handleRedeemRewardsButtonClick}>
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
