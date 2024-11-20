import { Box, useMediaQuery, useTheme } from "@mui/material";
import dynamic from "next/dynamic";
import Loading from "./Loading";
import { useEffect, useState } from "react";

const LineChart = dynamic(
  () => import("@nivo/line").then((m) => m.ResponsiveLine),
  {
    loading: () => <Loading />,
    ssr: false,
  }
);

interface IExplorerLineChartData {
  date_utc: string;
  greenLineNumericData: number;
  purpleLineNumericData: number;
  //   total_packets_dropped: number;
  //   total_stake: number;
}

interface IAxes {
  x: Date;
  y: number;
}

interface ILineAxes {
  id: string;
  data: Array<IAxes>;
}

export const PacketsLineChart = ({
  data,
}: {
  data: Array<IExplorerLineChartData>;
}) => {
  const theme = useTheme();
  const isDesktop = useMediaQuery(theme.breakpoints.up("lg"));

  const [chartData, setChartData] = useState<Array<ILineAxes>>();

  useEffect(() => {
    const resultData = transformData(data);
    if (resultData.length > 0) {
      setChartData(resultData);
    }
  }, []);

  const transformData = (data: Array<IExplorerLineChartData>) => {
    const greenLineData: ILineAxes = {
      id: "Numeric Data 1",
      data: [],
    };

    const purpleLineData: ILineAxes = {
      id: "Numeric Data 2",
      data: [],
    };

    data.map((item: any) => {
      const axesGreenLineData: IAxes = {
        x: new Date(item.date_utc),
        y: item.numericData1,
      };

      greenLineData.data.push(axesGreenLineData);

      const axesPurpleLineData: IAxes = {
        x: new Date(item.date_utc),
        y: item.numericData2,
      };

      purpleLineData.data.push(axesPurpleLineData);
    });
    return [{ ...purpleLineData }, { ...greenLineData }];
  };

  const yformat = (num: number | string | Date) => {
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
    } else {
      throw new Error("Unexpected value");
    }
  };
  return (
    <Box width={"100%"} height={isDesktop ? 200 : 150}>
      {chartData && (
        <LineChart
          curve="monotoneX"
          colors={["#8482FD", "#00CA33"]}
          data={chartData}
          animate
          enableSlices="x"
          margin={{
            bottom: 24,
            left: 36,
            right: 20,
            top: 20,
          }}
          theme={{
            grid: { line: { strokeWidth: 0 } },
            tooltip: { container: { color: "black" } },
            axis: {
              domain: {
                line: { stroke: "#ffffff", strokeWidth: 1, strokeOpacity: 1 },
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
          yScale={{ min: 150000000, type: "linear" }}
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
            tickValues: "every 2 days",
          }}
        />
      )}
    </Box>
  );
};
