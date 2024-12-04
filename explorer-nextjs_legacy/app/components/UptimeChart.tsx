import * as React from 'react';
import { CircularProgress, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { Chart } from 'react-google-charts';
import { format } from 'date-fns';
import { ApiState, UptimeStoryResponse } from '../typeDefs/explorer-api';

interface ChartProps {
  title?: string;
  xLabel: string;
  yLabel?: string;
  uptimeStory: ApiState<UptimeStoryResponse>;
  loading: boolean;
}

type FormattedDateRecord = [string, number];
type FormattedChartHeadings = string[];
type FormattedChartData = [FormattedChartHeadings | FormattedDateRecord];

export const UptimeChart: FCWithChildren<ChartProps> = ({ title, xLabel, yLabel, uptimeStory, loading }) => {
  const [formattedChartData, setFormattedChartData] = React.useState<FormattedChartData>();
  const theme = useTheme();
  const color = theme.palette.text.primary;
  React.useEffect(() => {
    if (uptimeStory.data?.history) {
      const allFormattedChartData: FormattedChartData = [['Date', 'Score']];
      uptimeStory.data.history.forEach((eachDate) => {
        const formattedDateUptimeRecord: FormattedDateRecord = [
          format(new Date(eachDate.date), 'MMM dd'),
          eachDate.uptime,
        ];
        allFormattedChartData.push(formattedDateUptimeRecord);
      });
      setFormattedChartData(allFormattedChartData);
    } else {
      const emptyData: any = [
        ['Date', 'Score'],
        ['Jul 27', 10],
      ];
      setFormattedChartData(emptyData);
    }
  }, [uptimeStory]);

  return (
    <>
      {title && <Typography>{title}</Typography>}
      {loading && <CircularProgress />}

      {!loading && uptimeStory && (
        <Chart
          style={{ minHeight: 480 }}
          chartType="LineChart"
          loader={<p>...</p>}
          data={
            uptimeStory.data
              ? formattedChartData
              : [
                  ['Date', 'Routing Score'],
                  [format(new Date(Date.now()), 'MMM dd'), 0],
                ]
          }
          options={{
            backgroundColor:
              theme.palette.mode === 'dark' ? theme.palette.nym.networkExplorer.background.tertiary : undefined,
            color: uptimeStory.error ? 'rgba(255, 255, 255, 0.4)' : 'rgba(255, 255, 255, 1)',
            colors: ['#FB7A21'],
            legend: {
              textStyle: {
                color,
                opacity: uptimeStory.error ? 0.4 : 1,
              },
            },

            intervals: { style: 'sticks' },
            hAxis: {
              // horizontal / date
              title: xLabel,
              titleTextStyle: {
                color,
              },
              textStyle: {
                color,
                // fontSize: 11
              },
              gridlines: {
                count: -1,
              },
            },
            vAxis: {
              // vertical / % Routing Score
              viewWindow: {
                min: 0,
                max: 100,
              },
              title: yLabel,
              titleTextStyle: {
                color,
                opacity: uptimeStory.error ? 0.4 : 1,
              },
              textStyle: {
                color,
                fontSize: 11,
                opacity: uptimeStory.error ? 0.4 : 1,
              },
            },
          }}
        />
      )}
    </>
  );
};

UptimeChart.defaultProps = {
  title: undefined,
};
