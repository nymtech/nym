import { Typography } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";

const TotalRewardsCard = () => {
  return (
    <ExplorerCard label="Total Rewards">
      <Typography
        variant="h3"
        sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
      >
        - NYM
      </Typography>
    </ExplorerCard>
  );
};

export default TotalRewardsCard;
