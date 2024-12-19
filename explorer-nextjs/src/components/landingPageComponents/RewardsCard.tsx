import { formatBigNum } from "@/utils/formatBigNumbers";
import { Stack, Typography } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";

export const RewardsCard = async () => {
  return (
    <ExplorerCard label="Operator rewards this month">
      <Stack>
        <Typography
          variant="h3"
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          {`${formatBigNum(10_000_111)} NYM`}
        </Typography>
      </Stack>
    </ExplorerCard>
  );
};
