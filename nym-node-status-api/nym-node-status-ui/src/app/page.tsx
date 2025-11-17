"use client";

import GraphCard from "@/components/GraphCard";
import { GatewayCanQueryMetadataTopup } from "@/components/graphs/GatewayCanQueryMetadataTopup";
import { GatewayDownloadSpeeds } from "@/components/graphs/GatewayDownloadSpeeds";
import { GatewayLoads } from "@/components/graphs/GatewayLoads";
import { GatewayPingPercentage } from "@/components/graphs/GatewayPingPercentage";
import { GatewayScores } from "@/components/graphs/GatewayScores";
import { GatewayUptimePercentage } from "@/components/graphs/GatewayUptimePercentage";
import { GatewayVersions } from "@/components/graphs/GatewayVersions";
import NestedLayoutWithHeader from "@/layouts/NestedLayoutWithHeader";
import Grid from "@mui/material/Grid";

export default function Home() {
  return (
    <NestedLayoutWithHeader>
      <Grid
        container
        spacing={2}
        columns={12}
        sx={{ mb: (theme) => theme.spacing(2) }}
      >
        <Grid size={{ xs: 12, sm: 8, lg: 4 }}>
          <GraphCard title="Gateway download speeds">
            <GatewayDownloadSpeeds />
          </GraphCard>
        </Grid>
        <Grid size={{ xs: 12, sm: 8, lg: 4 }}>
          <GraphCard title="Gateway ipv4 ping %">
            <GatewayPingPercentage />
          </GraphCard>
        </Grid>
        <Grid size={{ xs: 12, sm: 8, lg: 4 }}>
          <GraphCard title="Gateway Uptime %">
            <GatewayUptimePercentage />
          </GraphCard>
        </Grid>
        <Grid size={{ xs: 12, sm: 8, lg: 4 }}>
          <GraphCard title="Gateway scores">
            <GatewayScores />
          </GraphCard>
        </Grid>
        <Grid size={{ xs: 12, sm: 8, lg: 4 }}>
          <GraphCard title="Gateway loads">
            <GatewayLoads />
          </GraphCard>
        </Grid>
        <Grid size={{ xs: 12, sm: 8, lg: 4 }}>
          <GraphCard title="Gateway versions">
            <GatewayVersions />
          </GraphCard>
        </Grid>
        <Grid size={{ xs: 12, sm: 8, lg: 4 }}>
          <GraphCard title="Gateways with metadata top-up endpoint">
            <GatewayCanQueryMetadataTopup />
          </GraphCard>
        </Grid>
      </Grid>
    </NestedLayoutWithHeader>
  );
}
