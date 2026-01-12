import { useDVpnGatewaysTransformed } from "@/hooks/useGatewaysTransformed";
import Box from "@mui/material/Box";
import { BarChart } from "@mui/x-charts/BarChart";
import { rollup } from "d3-array";
import React from "react";

export const GatewayVersions = () => {
  const {
    query: { isSuccess, isError, data },
  } = useDVpnGatewaysTransformed();
  const binnedData = React.useMemo(() => {
    if (!isSuccess || data === undefined) {
      return undefined;
    }
    const versions = data.map((g) => g.build_information.build_version);
    // count occurrences of each version
    const versionCounts = rollup(
      versions,
      (v) => v.length,
      (v) => v, // group by version string
    );

    const chartData = Array.from(versionCounts, ([version, count]) => ({
      version,
      count,
    }));

    const labels = chartData.map((d) => d.version);
    const values = chartData.map((d) => d.count);

    return { labels, values };
  }, [data, isSuccess]);

  if (isError || !binnedData) {
    return null;
  }

  const { labels, values } = binnedData;

  return (
    <Box>
      <BarChart
        xAxis={[{ scaleType: "band", data: labels }]}
        series={[{ data: values }]}
        height={225}
      />
    </Box>
  );
};
