"use client";

import React, { useEffect, useState } from "react";
import { Box, Grid, Link, Typography } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import OpenInNewIcon from "@mui/icons-material/OpenInNew";
import { PeopleAlt } from "@mui/icons-material";
import { Title } from "@/app/components/Title";
import { StatsCard } from "@/app/components/StatsCard";
import { MixnodesSVG } from "@/app/icons/MixnodesSVG";
import { Icons } from "@/app/components/Icons";
import { GatewaysSVG } from "@/app/icons/GatewaysSVG";
import { ValidatorsSVG } from "@/app/icons/ValidatorsSVG";
import { ContentCard } from "@/app/components/ContentCard";
import { WorldMap } from "@/app/components/WorldMap";
import { BIG_DIPPER } from "@/app/api/constants";
import { formatNumber } from "@/app/utils";
import { useMainContext } from "./context/main";
import { useRouter } from "next/navigation";
import { ExplorerCard } from "./components/ExplorerCard";
import type { GetStaticProps, InferGetStaticPropsType } from "next";
import { ExplorerData, getCacheExplorerData } from "./api/explorer";
import { IExplorerLineChartData } from "./components/ExplorerLineChart";

// type ContentCardProps = {
//   overTitle?: string;
//   title?: string;
//   upDownLine?: ICardUpDownPriceLineProps;
//   titlePrice?: ICardTitlePriceProps;
//   dataRows?: ICardDataRowsProps;
//   graph?: Array<IExplorerLineChartData>;
//   progressBar?: IExplorerProgressBarProps;
//   paragraph?: string;
//   onClick?: ReactEventHandler;
// };

const explorerCard = {
  overTitle: "SINGLE",
  title: "SINGLE",
  upDownLine: {
    percentage: 10,
    numberWentUp: true,
  },
  titlePrice: {
    price: 1.15,
    upDownLine: {
      percentage: 10,
      numberWentUp: true,
    },
  },
  dataRows: {
    rows: [
      { key: "Market cap", value: "$ 1000000" },
      { key: "24H VOL", value: "$ 1000000" },
    ],
  },
  graph: {
    data: [
      {
        date_utc: "2024-11-20",
        numericData: 10,
      },
      {
        date_utc: "2024-11-21",
        numericData: 12,
      },
      {
        date_utc: "2024-11-22",
        numericData: 9,
      },
      {
        date_utc: "2024-11-23",
        numericData: 11,
      },
    ],
    color: "#00CA33",
    label: "Label",
  },

  paragraph: "Additional line",
};
export const DATA_REVALIDATE = 60;

export default function PageOverview() {
  const [explorerData, setExplorerData] = useState<ExplorerData | null>(null);
  const [noiseLineGraphData, setNoiseLineGraphData] = useState<{
    color: string;
    label: string;
    data: IExplorerLineChartData[];
  }>();
  const [stakeLineGraphData, setStakeLineGraphData] = useState<{
    color: string;
    label: string;
    data: IExplorerLineChartData[];
  }>();

  useEffect(() => {
    async function fetchData() {
      const data = await getCacheExplorerData();
      setExplorerData(data);
    }
    fetchData();
  }, []);

  const theme = useTheme();
  const router = useRouter();

  console.log("explorerData :>> ", explorerData);

  //  CURRENT EPOCH
  const currentEpochStart =
    explorerData?.currentEpochData.current_epoch_start || "";

  const progressBar = {
    title: "Current NGM epoch",
    start: currentEpochStart || "",
    showEpoch: true,
  };

  const formatBigNum = (num: number) => {
    if (typeof num === "number") {
      if (num >= 1000000000) {
        return (num / 1000000000).toFixed(1).replace(/\.0$/, "") + "B";
      }
      if (num >= 1000000) {
        return (num / 1000000).toFixed(1).replace(/\.0$/, "") + "M";
      }
      if (num >= 1000) {
        return (num / 1000).toFixed(1).replace(/\.0$/, "") + "K";
      }
      return num;
    }
  };

  // NOISE

  const noiseLast24H =
    explorerData?.packetsAndStakingData[
      explorerData.packetsAndStakingData.length - 1
    ].total_packets_sent +
    explorerData?.packetsAndStakingData[
      explorerData.packetsAndStakingData.length - 1
    ].total_packets_received;

  const noisePrevious24H =
    explorerData?.packetsAndStakingData[
      explorerData.packetsAndStakingData.length - 2
    ].total_packets_sent +
    explorerData?.packetsAndStakingData[
      explorerData.packetsAndStakingData.length - 2
    ].total_packets_received;

  const calculatePercentageChange = (last24H: number, previous24H: number) => {
    if (previous24H === 0) {
      throw new Error(
        "Cannot calculate percentage change when yesterday's value is zero."
      );
    }

    const change = ((last24H - previous24H) / previous24H) * 100;

    return parseFloat(change.toFixed(2));
  };

  const percentage = calculatePercentageChange(noiseLast24H, noisePrevious24H);

  const getPacketsData = () => {
    const data: Array<IExplorerLineChartData> = [];
    explorerData?.packetsAndStakingData.map((item: any) => {
      data.push({
        date_utc: item.date_utc,
        numericData: item.total_packets_sent + item.total_packets_received,
      });
    });
    return data;
  };

  useEffect(() => {
    const noiseLineGraphData = {
      color: "#8482FD",
      label: "Total packets sent and received",
      data: getPacketsData(),
    };
    setNoiseLineGraphData(noiseLineGraphData);
  }, [explorerData]);

  const noiseCard = {
    overTitle: "Noise generated last 24h",
    title: formatBigNum(noiseLast24H) || "",
    upDownLine: {
      percentage: Math.abs(percentage) || 0,
      numberWentUp: percentage > 0,
    },
    graph: noiseLineGraphData,
  };

  // STAKE

  const currentStake =
    Number(explorerData?.currentEpochRewardsData.interval.staking_supply) /
      1000000 || 0;

  const getStakeData = () => {
    const data: Array<IExplorerLineChartData> = [];
    explorerData?.packetsAndStakingData.map((item: any) => {
      data.push({
        date_utc: item.date_utc,
        numericData: item.total_stake / 1000000,
      });
    });
    return data;
  };

  useEffect(() => {
    const stakeLineGraphData = {
      color: "#00CA33",
      label: "Total stake delegated in NYM",
      data: getStakeData(),
    };
    setStakeLineGraphData(stakeLineGraphData);
  }, [explorerData]);

  const stakeCard = {
    overTitle: "Current network stake",
    title: currentStake + " NYM" || "",
    graph: stakeLineGraphData,
  };

  const {
    summaryOverview,
    gateways,
    validators,
    block,
    countryData,
    serviceProviders,
  } = useMainContext();
  return (
    <Box component="main" sx={{ flexGrow: 1 }}>
      <Grid>
        <Grid item paddingBottom={3}>
          <Title text="Overview" />
        </Grid>
        <Grid item>
          <Grid container spacing={3}>
            {summaryOverview && (
              <>
                <Grid item xs={12} md={4}>
                  <ExplorerCard {...explorerCard} />
                </Grid>
                <Grid item xs={12} md={4}>
                  <ExplorerCard progressBar={progressBar} />
                </Grid>
                <Grid item xs={12} md={4}>
                  <ExplorerCard {...noiseCard} />
                </Grid>
                <Grid item xs={12} md={4}>
                  <ExplorerCard {...stakeCard} />
                </Grid>
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() => router.push("/network-components/mixnodes")}
                    title="Mixnodes"
                    icon={<MixnodesSVG />}
                    count={summaryOverview.data?.mixnodes.count || ""}
                    errorMsg={summaryOverview?.error}
                  />
                </Grid>
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() =>
                      router.push("/network-components/mixnodes?status=active")
                    }
                    title="Active nodes"
                    icon={<Icons.Mixnodes.Status.Active />}
                    color={
                      theme.palette.nym.networkExplorer.mixnodes.status.active
                    }
                    count={summaryOverview.data?.mixnodes.activeset.active}
                    errorMsg={summaryOverview?.error}
                  />
                </Grid>
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() =>
                      router.push("/network-components/mixnodes?status=standby")
                    }
                    title="Standby nodes"
                    color={
                      theme.palette.nym.networkExplorer.mixnodes.status.standby
                    }
                    icon={<Icons.Mixnodes.Status.Standby />}
                    count={summaryOverview.data?.mixnodes.activeset.standby}
                    errorMsg={summaryOverview?.error}
                  />
                </Grid>
              </>
            )}
            {gateways && (
              <Grid item xs={12} md={4}>
                <StatsCard
                  onClick={() => router.push("/network-components/gateways")}
                  title="Gateways"
                  count={gateways?.data?.length || ""}
                  errorMsg={gateways?.error}
                  icon={<GatewaysSVG />}
                />
              </Grid>
            )}
            {serviceProviders && (
              <Grid item xs={12} md={4}>
                <StatsCard
                  onClick={() =>
                    router.push("/network-components/service-providers")
                  }
                  title="Service providers"
                  icon={<PeopleAlt />}
                  count={serviceProviders.data?.length}
                  errorMsg={summaryOverview?.error}
                />
              </Grid>
            )}
            {validators && (
              <Grid item xs={12} md={4}>
                <StatsCard
                  onClick={() => window.open(`${BIG_DIPPER}/validators`)}
                  title="Validators"
                  count={validators?.data?.count || ""}
                  errorMsg={validators?.error}
                  icon={<ValidatorsSVG />}
                />
              </Grid>
            )}
            {block?.data && (
              <Grid item xs={12}>
                <Link
                  href={`${BIG_DIPPER}/blocks`}
                  target="_blank"
                  rel="noreferrer"
                  underline="none"
                  color="inherit"
                  marginY={2}
                  paddingX={3}
                  paddingY={0.25}
                  fontSize={14}
                  fontWeight={600}
                  display="flex"
                  alignItems="center"
                >
                  <Typography fontWeight="inherit" fontSize="inherit">
                    Current block height is {formatNumber(block.data)}
                  </Typography>
                  <OpenInNewIcon
                    fontWeight="inherit"
                    fontSize="inherit"
                    sx={{ ml: 0.5 }}
                  />
                </Link>
              </Grid>
            )}
            <Grid item xs={12}>
              <ContentCard title="Distribution of nodes around the world">
                <WorldMap loading={false} countryData={countryData} />
              </ContentCard>
            </Grid>
          </Grid>
        </Grid>
      </Grid>
    </Box>
  );
}
