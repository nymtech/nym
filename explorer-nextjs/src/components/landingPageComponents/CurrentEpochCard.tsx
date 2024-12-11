import type { ExplorerData } from "@/app/api";
import { Box } from "@mui/material";
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
    <ExplorerCard title="Current NGM epoch">
      <ExplorerListItem
        value={
          <Box mt={3} width={"100%"}>
            <DynamicProgressBar {...progressBar} />
          </Box>
        }
      />
    </ExplorerCard>
  );
};
