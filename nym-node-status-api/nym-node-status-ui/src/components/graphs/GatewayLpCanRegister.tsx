import { useDVpnGatewaysTransformed } from "@/hooks/useGatewaysTransformed";
import Box from "@mui/material/Box";
import { BarChart } from "@mui/x-charts/BarChart";
import { rollup } from "d3-array";
import React from "react";

export const GatewayLpCanRegister = () => {
  const {
    query: { isSuccess, isError, data },
  } = useDVpnGatewaysTransformed();
  const binnedData = React.useMemo(() => {
    if (!isSuccess || data === undefined) {
      return undefined;
    }
    const results = data.map((g) => {
      const r = (g.last_probe?.outcome.lp as any)?.can_register;
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

    const bucketToLabel = { "yes": "success", "no": "failure", "-": "no score" } as const;
    const buckets = ["yes", "no", "-"] as const;
    const labels = buckets.map((b) => bucketToLabel[b]);
    const values = buckets.map((b) => resultCounts.get(b) ?? 0);

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
