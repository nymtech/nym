import type { ExplorerData } from "@/app/api";
import { Stack } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import { DynamicProgressBar } from "../progressBars/DynamicProgressBar";

interface ICurrentEpochCardProps {
  explorerData: ExplorerData | undefined;
}

export const CurrentEpochCard = (props: ICurrentEpochCardProps) => {
  const { explorerData } = props;

  const currentEpochStart =
    explorerData?.currentEpochData.current_epoch_start || "";

  const progressBar = {
    start: currentEpochStart || "",
    showEpoch: true,
  };
  return (
    <ExplorerCard title="Current NGM epoch">
      <Stack>
        <DynamicProgressBar {...progressBar} />
      </Stack>
    </ExplorerCard>
  );
};
