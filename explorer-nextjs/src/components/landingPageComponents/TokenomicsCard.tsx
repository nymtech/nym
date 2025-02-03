"use client";
import { fetchNymPrice } from "@/app/api";
import { Box, Skeleton, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import type { NymTokenomics } from "../../app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import { TitlePrice } from "../price/TitlePrice";

export const TokenomicsCard = () => {
  const {
    data: nymPrice,
    isLoading: isLoadingPrice,
    error: priceError,
  } = useQuery({
    queryKey: ["nymPrice"],
    queryFn: fetchNymPrice,
  });

  if (isLoadingPrice) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" sx={{ color: "pine.600", letterSpacing: 0.7 }}>
          Loading NYM Price...
        </Typography>
        <Skeleton variant="text" height={80} />
      </Stack>
    );
  }

  if (priceError || !nymPrice) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" sx={{ color: "pine.600", letterSpacing: 0.7 }}>
          Failed to load account balance.
        </Typography>
        <Skeleton variant="text" height={80} />
      </Stack>
    );
  }

  const nymPriceData: NymTokenomics = nymPrice;
  const nymPriceDataFormated = Number(nymPriceData.quotes.USD.price.toFixed(2));

  const titlePrice = {
    price: nymPriceDataFormated,
    // upDownLine: {
    //   percentage: 10,
    //   numberWentUp: true,
    // },
  };
  const marketCap = nymPriceData.quotes.USD.market_cap;
  const volume24H = nymPriceData.quotes.USD.volume_24h.toFixed(2);
  const dataRows = [
    { key: "Market cap", value: `$ ${marketCap}` },
    { key: "24H VOL", value: `$ ${volume24H}` },
  ];

  return (
    <ExplorerCard label="Tokenomics overview" sx={{ height: "100%" }}>
      <Stack gap={3} height="100%">
        <TitlePrice {...titlePrice} />
        <Box>
          {dataRows.map((row, i) => (
            <ExplorerListItem
              key={row.key}
              label={row.key}
              value={row.value}
              row={true}
              divider={i < dataRows.length - 1}
            />
          ))}
        </Box>
      </Stack>
    </ExplorerCard>
  );
};
