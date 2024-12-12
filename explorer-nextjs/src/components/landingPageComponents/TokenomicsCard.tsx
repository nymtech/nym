import type { ExplorerData, IPacketsAndStakingData } from "@/app/api";
import { Box, Stack, Typography } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import { MonoCard } from "../cards/MonoCard";
import { type ILineChartData, LineChart } from "../lineChart";
import ExplorerListItem from "../list/ListItem";
import { TitlePrice } from "../price/TitlePrice";

export const TokenomicsCard = () => {
  const titlePrice = {
    price: 1.15,
    upDownLine: {
      percentage: 10,
      numberWentUp: true,
    },
  };
  const dataRows = [
    { key: "Market cap", value: "$ 1000000" },
    { key: "24H VOL", value: "$ 1000000" },
  ];

  return (
    <ExplorerCard title="Tokenomics overview">
      <Stack>
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
