"use client";
import { fetchAccountBalance } from "@/app/api";
import { Box, Skeleton, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";
import ExplorerListItem from "../list/ListItem";
import { CardQRCode } from "../qrCode/QrCode";

interface IAccountInfoCardProps {
  address: string;
}

export const AccountInfoCard = (props: IAccountInfoCardProps) => {
  const { address } = props;

  const { data, isLoading, isError } = useQuery({
    queryKey: ["accountBalance", address],
    queryFn: () => fetchAccountBalance(address),
    enabled: !!address,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });

  if (isLoading) {
    return (
      <ExplorerCard label="Total NYM">
        <Stack gap={1}>
          <Skeleton variant="text" height={38} />
          <Skeleton variant="rectangular" height={128} width={128} />
          <Skeleton variant="text" height={300} />
        </Stack>
      </ExplorerCard>
    );
  }

  if (isError || !data) {
    return (
      <ExplorerCard label="Total NYM">
        <Typography variant="h5" sx={{ color: "pine.600", letterSpacing: 0.7 }}>
          Failed to account data.
        </Typography>
        <Skeleton variant="text" height={238} />
      </ExplorerCard>
    );
  }

  const balance =
    data.balances.length > 0 ? Number(data.total_value.amount) / 1000000 : 0;
  const balanceFormated = `${balance.toFixed(4)} NYM`;

  return (
    <ExplorerCard
      label="Total NYM"
      title={balanceFormated}
      sx={{ height: "100%" }}
    >
      <Stack gap={5}>
        <Box display={"flex"} justifyContent={"flex-start"}>
          <CardQRCode url={data.address} />
        </Box>

        <ExplorerListItem
          label="Address"
          value={
            <Stack
              direction="row"
              gap={0.1}
              alignItems="center"
              justifyContent="space-between"
              width="100%"
            >
              <Typography
                variant="body4"
                sx={{ wordWrap: "break-word", maxWidth: "85%" }}
              >
                {data.address}
              </Typography>
              <CopyToClipboard text={data.address} />
            </Stack>
          }
        />
      </Stack>
    </ExplorerCard>
  );
};
