import type { ExplorerData } from "@/app/api";
import ExplorerCard from "../Cards/ExplorerCard";
import ExplorerListItem from "../List/ListItem";
import { DynamicProgressBar } from "../progressBars/DynamicProgressBar";

interface ICurrentEpochCardProps {
  explorerData: ExplorerData | null;
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
    <div>
      <ExplorerCard title="Current NGM epoch">
        <ExplorerListItem value={<DynamicProgressBar {...progressBar} />} />
      </ExplorerCard>
    </div>
  );
};
