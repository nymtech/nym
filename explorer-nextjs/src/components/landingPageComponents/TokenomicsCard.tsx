import type { NymTokenomics } from "@/app/api/types";
import { NYM_PRICES_API } from "@/app/api/urls";
import { Box, Stack } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import { TitlePrice } from "../price/TitlePrice";

export const TokenomicsCard = async () => {
  const nymPrice = await fetch(NYM_PRICES_API, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });

  const nymPriceData: NymTokenomics = await nymPrice.json();
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
