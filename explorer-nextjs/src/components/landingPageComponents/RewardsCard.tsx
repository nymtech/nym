import { Stack, Typography } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";

export const RewardsCard = () => {
  return (
    <ExplorerCard title="Operator rewards this month">
      <Stack>
        <Typography
          variant="h3"
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          {"198.841720 NYM"}
        </Typography>
      </Stack>
    </ExplorerCard>
  );
};
