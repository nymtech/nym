"use client";

import { useChain } from "@cosmos-kit/react";

import type { ObservatoryBalance } from "@/app/api/types";
import { DATA_OBSERVATORY_BALANCES_URL } from "@/app/api/urls";
import { COSMOS_KIT_USE_CHAIN, NYM_MIXNET_CONTRACT } from "@/config";
import { useNymClient } from "@/hooks/useNymClient";
import { Button, Stack } from "@mui/material";
import type { Delegation } from "@nymproject/contract-clients/Mixnet.types";
import { useCallback, useEffect, useState } from "react";
import Loading from "../loading";
import InfoModal, { type InfoModalProps } from "../modal/InfoModal";
import RedeemRewardsModal from "../redeemRewards/RedeemRewardsModal";

const fee = { gas: "1000000", amount: [{ amount: "1000000", denom: "unym" }] };

const SubHeaderRowActions = () => {
  const [delegations, setDelegations] = useState<Delegation[]>([]);
  const [totalStakerRewards, setTotalStakerRewards] = useState<number>(0);
  const [openRedeemRewardsModal, setOpenRedeemRewardsModal] =
    useState<boolean>(false);

  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [infoModalProps, setInfoModalProps] = useState<InfoModalProps>({
    open: false,
  });
  const { address, nymClient } = useNymClient();
  const { getSigningCosmWasmClient } = useChain(COSMOS_KIT_USE_CHAIN);

  useEffect(() => {
    if (!nymClient || !address) return;

    const fetchDelegations = async () => {
      try {
        const data = await nymClient.getDelegatorDelegations({
          delegator: address,
        });
        setDelegations(data.delegations);
      } catch (error) {
        console.error("Failed to fetch delegations:", error);
      }
    };

    fetchDelegations();

    const fetchBalances = async () => {
      try {
        const data = await fetch(
          `${DATA_OBSERVATORY_BALANCES_URL}/${address}`,
          {
            headers: {
              Accept: "application/json",
              "Content-Type": "application/json; charset=utf-8",
            },
            next: { revalidate: 60 },
            // refresh event list cache at given interval
          },
        );
        const balances: ObservatoryBalance = await data.json();

        setTotalStakerRewards(balances.rewards.staking_rewards.amount);
      } catch (error) {
        console.error("Failed to fetch balances:", error);
      }
    };

    fetchBalances();
  }, [address, nymClient]);

  const handleRedeemRewards = useCallback(async () => {
    setIsLoading(true);
    setOpenRedeemRewardsModal(false);

    try {
      if (!nymClient || !address || !delegations.length) {
        throw new Error("Wallet, client, or delegations not available.");
      }

      const messages = delegations.map((delegation) => {
        const nodeId = delegation.node_id;

        // Generate the withdraw message
        const tx = {
          contractAddress: NYM_MIXNET_CONTRACT,
          funds: [],
          msg: {
            withdraw_delegator_reward: {
              node_id: nodeId,
            },
          },
        };

        return tx;
      });

      console.log("Messages prepared for multi-signing:", messages);

      const cosmWasmSigningClient = await getSigningCosmWasmClient();

      // Execute all messages in one transaction
      const result = await cosmWasmSigningClient.executeMultiple(
        address,
        messages,
        fee,
        "Redeeming all rewards",
      );

      console.log("Rewards redeemed successfully:", result);

      // Success state
      setIsLoading(false);
      setInfoModalProps({
        open: true,
        title: "Success",
        message: "All rewards have been redeemed successfully!",
        onClose: () => setInfoModalProps({ open: false }),
      });
      setOpenRedeemRewardsModal(false);
    } catch (e) {
      console.error("Error redeeming rewards:", e);
      setInfoModalProps({
        open: true,
        title: "Error",
        message:
          e instanceof Error
            ? e.message
            : "An error occurred while redeeming rewards.",
        onClose: () => setInfoModalProps({ open: false }),
      });
      setIsLoading(false);
    }
  }, [address, nymClient, delegations, getSigningCosmWasmClient]);

  const handleRedeemRewardsButtonClick = () => {
    setOpenRedeemRewardsModal(true);
  };

  if (!address || !nymClient) {
    return null;
  }

  return (
    <Stack direction="row" spacing={3} justifyContent={"end"}>
      <Button variant="contained" onClick={handleRedeemRewardsButtonClick}>
        Redeem NYM
      </Button>
      {isLoading && <Loading />}
      {openRedeemRewardsModal && (
        <RedeemRewardsModal
          onRedeem={() => handleRedeemRewards()}
          onClose={() => setOpenRedeemRewardsModal(false)}
          totalRewardsAmount={totalStakerRewards}
        />
      )}
      <InfoModal {...infoModalProps} />
    </Stack>
  );
};

export default SubHeaderRowActions;
