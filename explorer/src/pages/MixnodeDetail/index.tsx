import * as React from 'react';
import { Box, Grid, Typography } from '@mui/material';
import { MainContext } from 'src/context/main';
import { useParams, Link } from 'react-router-dom';
import { ContentCard } from 'src/components/ContentCard';
import { WorldMap } from 'src/components/WorldMap';
import { BondBreakdownTable } from 'src/components/BondBreakdown';
import { TwoColSmallTable } from 'src/components/TwoColSmallTable';
import { UptimeChart } from 'src/components/UptimeChart';
import { mixnodeToGridRow, scrollToRef } from 'src/utils';
import { ComponentError } from 'src/components/ComponentError';
import { UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MixNodeResponseItem } from 'src/typeDefs/explorer-api';
import { printableCoin } from '@nymproject/nym-validator-client';
import { GridRenderCellParams } from '@mui/x-data-grid';

const linkStyles = {
    color: 'inherit',
    textDecoration: 'none',
    marginLeft: 2,
}

const columns = [
    { field: 'owner', headerName: 'Owner', width: 380, },
    { field: 'identity_key', headerName: 'Identity Key', width: 420 },
    {
        field: 'bond',
        headerName: 'Bond',
        width: 180,
        renderCell: (params: GridRenderCellParams) => {
            const bondAsPunk = printableCoin({ amount: params.value as string, denom: 'upunk' })
            return (
                <Typography sx={linkStyles}>
                    {bondAsPunk}
                </Typography>
            )
        }
    },
    {
        field: 'host',
        headerName: 'IP:Port',
        width: 130,
    },
    {
        field: 'location',
        headerName: 'Location',
        width: 120,
    },
    {
        field: 'layer',
        headerName: 'Layer',
        width: 100,
        type: 'number',
    },
];

export const PageMixnodeDetail: React.FC = () => {
    const ref = React.useRef();
    const [row, setRow] = React.useState<MixNodeResponseItem[]>([]);
    const {
        fetchMixnodeById,
        mixnodeDetailInfo,
        fetchStatsById,
        fetchDelegationsById,
        fetchUptimeStoryById,
        fetchStatusById,
        stats,
        status,
        uptimeStory,
    } = React.useContext(MainContext);
    let { id }: any = useParams();

    React.useEffect(() => {
        const hasNoDetail = id && !mixnodeDetailInfo;
        const hasIncorrectDetail = id && mixnodeDetailInfo?.data && mixnodeDetailInfo?.data[0].mix_node.identity_key !== id;
        if (hasNoDetail || hasIncorrectDetail) {
            fetchMixnodeById(id)
            fetchDelegationsById(id)
            fetchStatsById(id)
            fetchStatusById(id)
            fetchUptimeStoryById(id)
        } else {
            if (mixnodeDetailInfo?.data !== undefined) {
                setRow(mixnodeDetailInfo?.data);
            }
        }
    }, [id, mixnodeDetailInfo]);



    React.useEffect(() => {
        scrollToRef(ref);
    }, [ref])

    return (
        <>
            <Box
                component='main'
                ref={ref}
            >
                <Grid container spacing={2}>
                    <Grid item xs={12}>
                        <Typography>
                            Mixnode Detail
                        </Typography>
                    </Grid>
                    <Grid item xs={12} xl={9}>
                        {mixnodeDetailInfo && (
                            <UniversalDataGrid
                                columnsData={columns}
                                rows={mixnodeToGridRow(row)}
                                loading={mixnodeDetailInfo.isLoading}
                                height={180}
                                pageSize='5'
                            />
                        )}
                    </Grid>
                    <Grid item xs={12} xl={9}>
                        <ContentCard title='Bond Breakdown'>
                            <BondBreakdownTable />
                        </ContentCard>
                    </Grid>
                </Grid>
                <Grid
                    container
                    spacing={2}
                    sx={{ marginTop: 1 }}
                >
                    <Grid
                        item
                        xs={12}
                        md={4}
                        xl={3}
                    >

                        <ContentCard title='Mixnode Stats'>
                            {stats && (
                                <>
                                    {stats.error && <ComponentError text='There was a problem retrieving this nodes stats.' />}
                                    <TwoColSmallTable
                                        loading={stats.isLoading}
                                        error={stats?.error?.message}
                                        title='Since startup'
                                        keys={['Received', 'Sent', 'Explicitly dropped']}
                                        values={
                                            [
                                                stats?.data?.packets_received_since_startup || 0,
                                                stats?.data?.packets_sent_since_startup || 0,
                                                stats?.data?.packets_explicitly_dropped_since_startup || 0,
                                            ]
                                        }
                                    />
                                    <TwoColSmallTable
                                        loading={stats.isLoading}
                                        error={stats?.error?.message}
                                        title='Since last update'
                                        keys={['Received', 'Sent', 'Explicitly dropped']}
                                        values={
                                            [
                                                stats?.data?.packets_received_since_last_update || 0,
                                                stats?.data?.packets_sent_since_last_update || 0,
                                                stats?.data?.packets_explicitly_dropped_since_last_update || 0,
                                            ]
                                        }
                                        marginBottom
                                    />
                                </>
                            )}
                            {!stats && <Typography>No stats information</Typography>}
                        </ContentCard>
                    </Grid>
                    <Grid
                        item
                        xs={12}
                        md={8}
                        xl={6}
                    >
                        {uptimeStory && (
                            <ContentCard title='Uptime story'>
                                {uptimeStory.error && <ComponentError text='There was a problem retrieving uptime history.' />}
                                <UptimeChart
                                    loading={uptimeStory.isLoading}
                                    xLabel='date'
                                    yLabel='uptime'
                                    uptimeStory={uptimeStory}
                                />
                            </ContentCard>
                        )}
                    </Grid>
                </Grid>
                <Grid container spacing={2} sx={{ marginTop: 1 }}>
                    <Grid item xs={12} md={4} xl={3}>
                        {status && (
                            <ContentCard title='Mixnode Status'>
                                {status.error && <ComponentError text='There was a problem retrieving port information' />}
                                <TwoColSmallTable
                                    loading={status.isLoading}
                                    error={status?.error?.message}
                                    keys={['Mix port', 'Verloc port', 'HTTP port']}
                                    values={[1789, 1790, 8000].map(each => each)}
                                    icons={status?.data?.ports && Object.values(status.data.ports) || [false, false, false]}
                                />
                            </ContentCard>
                        )}
                    </Grid>
                    <Grid item xs={12} md={8} xl={6}>
                        {mixnodeDetailInfo && (
                            <ContentCard title='Location'>
                                {mixnodeDetailInfo?.error && <ComponentError text='There was a problem retrieving this mixnode location' />}
                                {mixnodeDetailInfo.data && mixnodeDetailInfo?.data[0]?.location && (
                                    <WorldMap
                                        loading={mixnodeDetailInfo.isLoading}
                                        title='Location'
                                        userLocation={
                                            [mixnodeDetailInfo?.data[0]?.location?.lng, mixnodeDetailInfo?.data[0]?.location?.lat]
                                        }
                                    />
                                )}
                            </ContentCard>
                        )}
                    </Grid>
                </Grid>
            </Box>
        </>
    )
}
