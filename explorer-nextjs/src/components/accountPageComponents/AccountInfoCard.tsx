"use client";
import type { IAccountBalancesInfo } from "@/app/api/types";
import { Box, Stack, Typography } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";
import ExplorerListItem from "../list/ListItem";
import { CardQRCode } from "../qrCode/QrCode";

interface IAccountInfoCardProps {
  accountInfo: IAccountBalancesInfo;
}

export const AccountInfoCard = (props: IAccountInfoCardProps) => {
  const { accountInfo } = props;

  const balance = Number(accountInfo.balances[0].amount) / 1000000;
  const balanceFormated = `${balance} NYM`;

  return (
    <ExplorerCard
      label="Address"
      title={balanceFormated}
      sx={{ height: "100%" }}
    >
      <Stack gap={5}>
        <Box display={"flex"} justifyContent={"flex-start"}>
          <CardQRCode url={accountInfo.address} />
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
              <Typography variant="body4">{accountInfo.address}</Typography>
              <CopyToClipboard text={accountInfo.address} />
            </Stack>
          }
        />
      </Stack>
    </ExplorerCard>
  );
};
