import { Skeleton, Stack, type SxProps } from "@mui/material";
import ExplorerCard from "./ExplorerCard";

const CardSkeleton = ({ sx }: { sx?: SxProps }) => {
  return (
    <ExplorerCard label={<Skeleton variant="text" width={200} />} sx={sx}>
      <Stack gap={1}>
        <Skeleton variant="text" />
        <Skeleton variant="rounded" height={75} />
        <Skeleton variant="text" />
        <Skeleton variant="text" />
      </Stack>
    </ExplorerCard>
  );
};

export default CardSkeleton;
