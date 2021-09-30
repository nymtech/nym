import * as React from 'react';
import Paper from '@mui/material/Paper';
import { Typography } from '@mui/material';
import { Chart } from 'react-google-charts';
import { MainContext } from '../context/main';
import { ApiState, UptimeStoryResponse } from 'src/typeDefs/explorer-api';
import { format } from 'date-fns';
interface ChartProps {
    title?: string
    xLabel: string
    yLabel: string
    uptimeStory: ApiState<UptimeStoryResponse>
}

type FormattedDateRecord = [string, number, number];
type FormattedChartHeadings = [string, string, string];
type FormattedChartData = [FormattedChartHeadings | FormattedDateRecord];

export function UptimeChart({ title, xLabel, yLabel, uptimeStory }: ChartProps) {

    const [formattedChartData, setFormattedChartData] = React.useState<FormattedChartData>()
    const { mode }: any = React.useContext(MainContext);

    React.useEffect(() => {
        if (uptimeStory.data?.history) {
            let allFormattedChartData: FormattedChartData = [
                ["Date", "UptimeV4", "UptimeV6"],
            ];
            uptimeStory.data.history.map((eachDate) => {
                const formattedDateUptimeRecord: FormattedDateRecord = [
                    format(new Date(eachDate.date), "MMM dd"),
                    eachDate.ipv4_uptime,
                    eachDate.ipv6_uptime
                ]
                allFormattedChartData.push(formattedDateUptimeRecord);
            });
            setFormattedChartData(allFormattedChartData)
        }
    }, [])

    return (
        <>
            {title && (
                <Typography>
                    {title}
                </Typography>
            )}

            <Chart
                style={{ minHeight: 480 }}
                chartType="LineChart"
                loader={<div>Loading...</div>}
                data={formattedChartData}
                options={{
                    backgroundColor: mode === 'dark' ? 'rgb(50, 60, 81)' : 'rgb(241, 234, 234)',
                    color: 'white',
                    colors: ['#FB7A21', "#CC808A"],
                    legend: {
                        textStyle: { color: 'white' }
                    },

                    intervals: { style: 'sticks' },
                    hAxis: { // horizontal / date
                        title: xLabel,
                        titleTextStyle: {
                            color: mode === 'dark' ? 'white' : 'black',
                        },
                        textStyle: {
                            color: mode === 'dark' ? 'white' : 'black',
                            // fontSize: 11
                        },
                        gridlines: {
                            count: -1,
                        }

                    },
                    vAxis: { // % uptime vertical
                        viewWindow: {
                            min: 0,
                            max: 100,
                        },
                        title: yLabel,
                        titleTextStyle: {
                            color: mode === 'dark' ? 'white' : 'black',
                        },
                        textStyle: {
                            color: mode === 'dark' ? 'white' : 'black',
                            fontSize: 11,
                        },
                    }
                }}
            />
        </>
    );
}
