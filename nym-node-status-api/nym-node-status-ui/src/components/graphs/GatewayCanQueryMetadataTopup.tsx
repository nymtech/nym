import { useDVpnGatewaysTransformed } from "@/hooks/useGatewaysTransformed";
import Box from "@mui/material/Box";
import { BarChart } from "@mui/x-charts/BarChart";
import { rollup } from "d3-array";
import React from "react";

export const GatewayCanQueryMetadataTopup = () => {
  const {
    query: { isSuccess, isError, data },
  } = useDVpnGatewaysTransformed();
  const binnedData = React.useMemo(() => {
    if (!isSuccess || data === undefined) {
      return undefined;
    }
    const results = data.map((g) => {
      const r = (g.last_probe?.outcome.wg as any)?.can_query_metadata_v4;
      if (r === undefined) {
        return "-";
      }
      if (r === true) {
        return "yes";
      }
      return "no";
    });
    // count occurrences of each result
    const resultCounts = rollup(
      results,
      (v) => v.length,
      (v) => v, // group by result string
    );

    const chartData = Array.from(resultCounts, ([result, count]) => ({
      result,
      count,
    }));

    const labels = chartData.map((d) => d.result);
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
