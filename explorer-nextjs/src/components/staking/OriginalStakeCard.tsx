"use client";

import { useNymClient } from "@/hooks/useNymClient";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Typography } from "@mui/material";
import { useEffect, useState } from "react";
import ExplorerCard from "../cards/ExplorerCard";

const OriginalStakeCard = () => {
  const [origialStake, setOriginalStake] = useState(0);
  const { nymClient, address } = useNymClient();

  useEffect(() => {
    const getDelegations = async () => {
      if (!nymClient || !address) return;

      const delegations = await nymClient?.getDelegatorDelegations({
        delegator: address,
      });

      const totaluNYMStake = delegations.delegations.reduce((acc, curr) => {
        return acc + Number(curr.amount.amount);
      }, 0);

      setOriginalStake(totaluNYMStake);
    };
    getDelegations();
  }, [address, nymClient]);
  return (
    <ExplorerCard label="Original Stake">
      <Typography
        variant="h3"
        sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
      >
        {`${formatBigNum(origialStake / 1_000_000)} NYM`}
      </Typography>
    </ExplorerCard>
  );
};

export default OriginalStakeCard;
