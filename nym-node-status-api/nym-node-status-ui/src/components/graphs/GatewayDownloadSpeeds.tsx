import { useDVpnGatewaysTransformed } from "@/hooks/useGatewaysTransformed";
import Box from "@mui/material/Box";
import { BarChart } from "@mui/x-charts/BarChart";
import { bin } from "d3-array";
import React from "react";

export const GatewayDownloadSpeeds = () => {
  const {
    query: { isSuccess, isError, data },
  } = useDVpnGatewaysTransformed();
  const binnedData = React.useMemo(() => {
    if (!isSuccess || data === undefined) {
      return undefined;
    }
    const binner = bin().thresholds(10); // Number of bins
    const bins = binner(
      data
        .map((g) => g.extra.downloadSpeedMBPerSec)
        .filter((g) => Boolean(g)) as number[],
    );

    const labels = bins.map((b) => `${b.x0}-${b.x1} MB/sec`);
    const values = bins.map((b) => b.length); // count per bin

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
