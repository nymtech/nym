"use client";
import { ResponsiveLine } from "@nivo/line";

export interface ILineChartData {
  date_utc: string;
  numericData?: number;
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
  const chartData: ILineAxes = {
    id: label,
    data: data.map((item) => ({
      x: new Date(item.date_utc), // Convert date string to Date object
      y: item.numericData || 0, // Default to 0 if numericData is missing
    })),
  };

  // **Find the highest Y value and add a 10% buffer**
  const maxYValue =
    Math.max(...chartData.data.map((point) => point.y)) * 1.1 || 150000000;

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
    <ResponsiveLine
      curve="basis"
      colors={[color]}
      data={[chartData]}
      animate
      enablePoints={false}
      enableSlices="x"
      margin={{
        bottom: 24,
        left: 36,
        right: 18,
        top: 20,
      }}
      theme={{
        grid: { line: { strokeWidth: 0 } },
        tooltip: {
          container: {
            color: "black",
            fontSize: 10,
            maxWidth: 250,
            lineHeight: 1,
          },
        },
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
      yScale={{
        min: 0, // Keeping the minimum static
        max: maxYValue, // **Dynamically set max value**
        type: "linear",
      }}
      xFormat="time:%Y-%m-%d"
      axisLeft={{
        legendOffset: 12,
        tickSize: 3,
        format: yformat,
        tickValues: 6,
      }}
      axisBottom={{
        format: "%b %d",
        legendOffset: -12,
        tickValues: chartData.data.length > 7 ? "every 6 days" : "every 2 days",
      }}
    />
  );
};
