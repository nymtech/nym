import { useDVpnGatewaysTransformed } from "@/hooks/useGatewaysTransformed";
import Box from "@mui/material/Box";
import { BarChart } from "@mui/x-charts/BarChart";
import React from "react";

export const GatewayLoads = () => {
  const {
    query: { isSuccess, isError, data },
  } = useDVpnGatewaysTransformed();
  const binnedData = React.useMemo(() => {
    if (!isSuccess || data === undefined) {
      return undefined;
    }
    const binned = data.reduce(
      (acc, g) => {
        const score: "low" | "medium" | "high" | "offline" =
          (g as any).performance_v2?.load || "offline";
        acc[score] += 1;
        return acc;
      },
      { offline: 0, low: 0, medium: 0, high: 0 },
    );

    const labels = ["offline", "low", "medium", "high"];
    const values = Object.values(binned);

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
