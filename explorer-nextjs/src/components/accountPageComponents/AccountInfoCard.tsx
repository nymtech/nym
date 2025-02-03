"use client";
import { fetchAccountBalance } from "@/app/api";
import { Box, Stack, Typography } from "@mui/material";
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
  });

  if (isLoading) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" fontWeight="light">
          Loading account balance...
        </Typography>
      </Stack>
    );
  }

  if (isError || !data) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" fontWeight="light">
          Failed to load account balance.
        </Typography>
      </Stack>
    );
  }

  const balance = Number(data.balances[0].amount) / 1000000;
  const balanceFormated = `${balance} NYM`;

  return (
    <ExplorerCard
      label=""
      title={balanceFormated}
      sx={{ height: "100%", pt: 0 }}
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
              <Typography variant="body4">{data.address}</Typography>
              <CopyToClipboard text={data.address} />
            </Stack>
          }
        />
      </Stack>
    </ExplorerCard>
  );
};
