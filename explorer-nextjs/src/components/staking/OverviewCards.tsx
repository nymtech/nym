"use client";

import { useNymClient } from "@/hooks/useNymClient";
import { Grid2 } from "@mui/material";
import OriginalStakeCard from "./OriginalStakeCard";
import TotalRewardsCard from "./TotalRewardsCard";
import TotalStakeCard from "./TotalStakeCard";

const OverviewCards = () => {
  const { address } = useNymClient();

  if (!address) {
    return null;
  }

  return (
    <Grid2 container spacing={3}>
      <Grid2
        size={{
          xs: 12,
          md: 4,
        }}
      >
        <TotalStakeCard />
      </Grid2>
      <Grid2
        size={{
          xs: 12,
          md: 4,
        }}
      >
        <OriginalStakeCard />
      </Grid2>
      <Grid2
        size={{
          xs: 12,
          md: 4,
        }}
      >
        <TotalRewardsCard />
      </Grid2>
    </Grid2>
  );
};

export default OverviewCards;
