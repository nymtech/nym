import * as React from 'react';
import Paper from '@mui/material/Paper';
import { Typography } from '@mui/material';
import { Chart } from 'react-google-charts';
import { MainContext } from '../context/main';
interface ChartProps {
    title?: string
    xLabel: string
    yLabel: string
}

export function UptimeChart({ title, xLabel, yLabel }: ChartProps) {
    const { mode }: any = React.useContext(MainContext);
        return (
            <>
                {title && (
                    <Typography>
                        {title}
                    </Typography>
                )}
                
                <Chart
                    style={{ minHeight: 480, border: '1px solid red' }}
                    chartType="LineChart"
                    loader={<div>Loading...</div>}
                    data={
                        [
                            [
                                { type: 'number', label: 'x' },
                                { type: 'number', label: 'values' },
                                { id: 'i0', type: 'number', role: 'interval' },
                                { id: 'i1', type: 'number', role: 'interval' },
                                { id: 'i2', type: 'number', role: 'interval' },
                                { id: 'i2', type: 'number', role: 'interval' },
                                { id: 'i2', type: 'number', role: 'interval' },
                                { id: 'i2', type: 'number', role: 'interval' },
                            ],
                                [1, 100, 90, 110, 85, 96, 104, 120],
                                [2, 120, 95, 130, 90, 113, 124, 140],
                                [3, 130, 105, 140, 100, 117, 133, 139],
                                [4, 90, 85, 95, 85, 88, 92, 95],
                                [5, 70, 74, 63, 67, 69, 70, 72],
                                [6, 30, 39, 22, 21, 28, 34, 40],
                                [7, 80, 77, 83, 70, 77, 85, 90],
                                [8, 100, 90, 110, 85, 95, 102, 110],
                        ]
                    }
                    options={{
                        backgroundColor: mode === 'dark' ? 'rgb(50, 60, 81)' : 'rgb(241, 234, 234)',
                        color: 'white',
                        colors: ['#FB7A21'],
                        intervals: { style: 'sticks' },
                        legend: 'none',
                        hAxis: {
                            title: xLabel,
                            titleTextStyle: {
                                color: mode === 'dark' ? 'white' : 'black',
                            },
                            gridlines: {
                                count: -1,
                            }

                        },
                        vAxis: {
                            title: yLabel,
                            titleTextStyle: {
                                color: mode === 'dark' ? 'white' : 'black',
                            }
                        }
                    }}
                />
            </>
        );
}
