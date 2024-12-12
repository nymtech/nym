import type { ExplorerData } from "@/app/api";
import { MonoCard } from "../cards/MonoCard";

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
  return <MonoCard progressBar={progressBar} overTitle="Current NGM epoch" />;
};
