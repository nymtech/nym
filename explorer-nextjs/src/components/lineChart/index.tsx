"use client";
import { Box, useMediaQuery, useTheme } from "@mui/material";
import dynamic from "next/dynamic";
import { useEffect, useState } from "react";
import Loading from "../loading";

const NivoLineChart = dynamic(
  () => import("@nivo/line").then((m) => m.ResponsiveLine),
  {
    loading: () => <Loading />,
    ssr: false,
  },
);

export interface ILineChartData {
  date_utc: string;
  numericData?: number;
  // purpleLineNumericData?: number;
}

interface IAxes {
  x: Date;
  y: number;
}

interface ILineAxes {
  id: string;
  data: Array<IAxes>;
}

export const LineChart = ({
  data,
  color,
  label,
}: {
  data: Array<ILineChartData>;
  color: string;
  label: string;
}) => {
  const theme = useTheme();
  const isDesktop = useMediaQuery(theme.breakpoints.up("lg"));

  const [chartData, setChartData] = useState<Array<ILineAxes>>();

  useEffect(() => {
    const resultData = transformData(data);
    if (resultData.length > 0) {
      setChartData(resultData);
    }
  }, [data]);

  const transformData = (data: Array<ILineChartData>) => {
    const lineData: ILineAxes = {
      id: label,
      data: [],
    };

    // const purpleLineData: ILineAxes = {
    //   id: "Numeric Data 2",
    //   data: [],
    // };

    data.map((item: ILineChartData) => {
      const axesGreenLineData: IAxes = {
        x: new Date(item.date_utc),
        y: item.numericData || 0,
      };

      lineData.data.push(axesGreenLineData);

      // const axesPurpleLineData: IAxes = {
      //   x: new Date(item.date_utc),
      //   y: item.purpleLineNumericData,
      // };

      // purpleLineData.data.push(axesPurpleLineData);
    });
    return [{ ...lineData }];
  };

  const yformat = (num: number | string | Date) => {
    if (typeof num === "number") {
      if (num >= 1000000000) {
        return `${(num / 1000000000).toFixed(1).replace(/\.0$/, "")}B`;
      }
      if (num >= 1000000) {
        return `${(num / 1000000).toFixed(1).replace(/\.0$/, "")}M`;
      }
      if (num >= 1000) {
        return `${(num / 1000).toFixed(1).replace(/\.0$/, "")}K`;
      }
      return num;
    }
    throw new Error("Unexpected value");
  };

  return (
    <Box width={"100%"} height={isDesktop ? 200 : 150}>
      {chartData && (
        <NivoLineChart
          curve="monotoneX"
          colors={[color]}
          data={chartData}
          animate
          enableSlices="x"
          margin={{
            bottom: 24,
            left: 36,
            right: 16,
            top: 20,
          }}
          theme={{
            grid: { line: { strokeWidth: 0 } },
            tooltip: { container: { color: "black" } },
            axis: {
              domain: {
                line: { stroke: "#C3D7D7", strokeWidth: 1, strokeOpacity: 1 },
              },
              ticks: {
                text: {
                  fill: "#818386",
                },
              },
              legend: {
                text: {
                  fill: "#818386",
                },
              },
            },
          }}
          xScale={{
            type: "time",
            format: "%Y-%m-%d",
          }}
          yScale={{ min: 1, type: "linear" }}
          xFormat="time:%Y-%m-%d"
          axisLeft={{
            legendOffset: 12,
            tickSize: 3,
            format: yformat,
            tickValues: 5,
          }}
          axisBottom={{
            format: "%b %d",
            legendOffset: -12,
            tickValues:
              chartData[0].data.length > 7 ? "every 4 days" : "every day",
          }}
        />
      )}
    </Box>
  );
};
