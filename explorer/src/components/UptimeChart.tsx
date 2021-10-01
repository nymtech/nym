import * as React from 'react';
import Paper from '@mui/material/Paper';
import { CircularProgress, Typography } from '@mui/material';
import { Chart } from 'react-google-charts';
import { MainContext } from '../context/main';
import { ApiState, UptimeStoryResponse } from 'src/typeDefs/explorer-api';
import { format } from 'date-fns';
interface ChartProps {
    title?: string
    xLabel: string
    yLabel: string
    uptimeStory: ApiState<UptimeStoryResponse>
    loading: boolean
}

type FormattedDateRecord = [string, number, number];
type FormattedChartHeadings = [string, string, string];
type FormattedChartData = [FormattedChartHeadings | FormattedDateRecord];

export function UptimeChart({ title, xLabel, yLabel, uptimeStory, loading }: ChartProps) {

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
            console.log("A) setting allFormattedChartData to this ===>", allFormattedChartData);
            setFormattedChartData(allFormattedChartData)
        } else {
            const emptyData: any = [
                ["Date", "UptimeV4", "UptimeV6"],
                ['Jul 27', 10, 10]
            ];
            console.log("B) setting allFormattedChartData to this ===>", emptyData);
            setFormattedChartData(emptyData)
        }
    }, [uptimeStory])

    console.log('uptimeStory =======> ', uptimeStory)
    console.log('formattedChartData =======> ', formattedChartData)

    return (
        <>
            {title && (
                <Typography>
                    {title}
                </Typography>
            )}
            {loading && <CircularProgress />}

            {!loading && uptimeStory && (
                <Chart
                    style={{ minHeight: 480 }}
                    chartType="LineChart"
                    loader={<p>...</p>}
                    data={uptimeStory.data ? formattedChartData : [["Date", "UptimeV4", "UptimeV6"], [format(new Date(Date.now()), "MMM dd"), 0, 0]]}
                    options={{
                        backgroundColor: mode === 'dark' ? 'rgb(50, 60, 81)' : 'rgb(241, 234, 234)',
                        color: uptimeStory.error ? `rgba(255, 255, 255, 0.4)` : `rgba(255, 255, 255, 1)`,
                        colors: ['#FB7A21', "#CC808A"],
                        legend: {
                            textStyle: {
                                color: 'white',
                                opacity: uptimeStory.error ? 0.4 : 1,
                            }
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
                                opacity: uptimeStory.error ? 0.4 : 1,
                            },
                            textStyle: {
                                color: mode === 'dark' ? 'white' : 'black',
                                fontSize: 11,
                                opacity: uptimeStory.error ? 0.4 : 1,
                            },
                        }
                    }}
                />
            )}
        </>
    );
}
