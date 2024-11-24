"use client";

import React, { useEffect } from "react";
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
    priceWentUp: true,
  },
  titlePrice: {
    price: 1.15,
    upDownLine: {
      percentage: 10,
      priceWentUp: true,
    },
  },
  dataRows: {
    rows: [
      { key: "Market cap", value: "$ 1000000" },
      { key: "24H VOL", value: "$ 1000000" },
    ],
  },
  graph: [
    {
      date_utc: "2024-11-20",
      greenLineNumericData: 10,
      purpleLineNumericData: 5,
    },
    {
      date_utc: "2024-11-21",
      greenLineNumericData: 12,
      purpleLineNumericData: 6,
    },
    {
      date_utc: "2024-11-22",
      greenLineNumericData: 9,
      purpleLineNumericData: 5,
    },
    {
      date_utc: "2024-11-23",
      greenLineNumericData: 11,
      purpleLineNumericData: 4,
    },
  ],
  progressBar: {
    title: "Current NGM epoch",
    start: "2024-11-24T13:26:19Z",
    end: "2024-11-24T14:26:19Z",
    showEpoch: true,
  },
  paragraph: "Additional line",
};

const PageOverview = () => {
  const theme = useTheme();
  const router = useRouter();

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
};

export default PageOverview;
