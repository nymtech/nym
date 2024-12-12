import type { CurrentEpochData } from "@/app/api";
import { CURRENT_EPOCH } from "@/app/api/urls";
import { Stack } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import EpochProgressBar from "../progressBars/EpochProgressBar";

export const CurrentEpochCard = async () => {
  const data = await fetch(CURRENT_EPOCH, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });

  const currentEpochData: CurrentEpochData = await data.json();

  if (!currentEpochData) {
    return null;
  }

  const currentEpochStart = currentEpochData.current_epoch_start || "";

  const progressBar = {
    start: currentEpochStart || "",
    showEpoch: true,
  };
  return (
    <ExplorerCard label="Current NGM epoch">
      <Stack>
        <EpochProgressBar {...progressBar} />
      </Stack>
    </ExplorerCard>
  );
};
