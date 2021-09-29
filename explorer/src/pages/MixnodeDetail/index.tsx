import * as React from 'react';
import { Box, Grid, Typography } from '@mui/material';
import { MixnodesTable } from '../../components/MixnodesTable';
import { MainContext } from 'src/context/main';
import { useParams } from 'react-router-dom';
import { ContentCard } from 'src/components/ContentCard';
import { WorldMap } from 'src/components/WorldMap';
import { BondBreakdownTable } from 'src/components/BondBreakdown';
import { TwoColSmallTable } from 'src/components/TwoColSmallTable';
import { UptimeChart } from 'src/components/UptimeChart';
import { scrollToRef } from 'src/utils';

export const PageMixnodeDetail: React.FC = () => {
    const ref = React.useRef();
    const { 
        fetchMixnodeById,
        mixnodeDetailInfo,
        fetchDelegationsById,
        fetchStatsById,
        stats
    } = React.useContext(MainContext);
    let { id }: any = useParams();

    React.useEffect(() => {
        const hasNoDetail = id && !mixnodeDetailInfo;
        const hasIncorrectDetail = id && mixnodeDetailInfo?.data && mixnodeDetailInfo?.data[0].mix_node.identity_key !== id;
        // 1. if we have no specific ID/mixnode selected
        // OR the detail on page is different to what is in state, fetch mixnodeDetailInfo & delegates info
        if (hasNoDetail || hasIncorrectDetail) {
            fetchMixnodeById(id)
            // 2. delegations for Bond breakdown table: owner, amount, block_height
            fetchDelegationsById(id)
            fetchStatsById(id)
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
                        { mixnodeDetailInfo && (
                            <MixnodesTable mixnodes={mixnodeDetailInfo} />
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

                        { stats && stats.data && (
                            <ContentCard title='Mixnode Stats'>
                                
                                <TwoColSmallTable
                                    title='Since startup'
                                    keys={['Received', 'Sent', 'Explicitly dropped']}
                                    values={
                                        [ 
                                            stats?.data?.packets_received_since_startup,
                                            stats?.data?.packets_sent_since_startup,
                                            stats?.data?.packets_explicitly_dropped_since_startup
                                        ]
                                    }
                                />
                                
                                <TwoColSmallTable
                                    title='Since last update'
                                    keys={['Received', 'Sent', 'Explicitly dropped']}
                                    values={
                                        [ 
                                            stats?.data?.packets_received_since_last_update,
                                            stats?.data?.packets_sent_since_last_update,
                                            stats?.data?.packets_explicitly_dropped_since_last_update
                                        ]
                                    }
                                    marginBottom
                                />
                            </ContentCard>
                        )}
                    </Grid>
                    <Grid
                        item
                        xs={12}
                        md={8}
                        xl={6}
                    >
                            <ContentCard title='Uptime story'>
                                <UptimeChart
                                    xLabel='months'
                                    yLabel='nodes'
                                />
                            </ContentCard>
                    </Grid>
                </Grid>
                <Grid container spacing={2} sx={{ marginTop: 1 }}>
                    <Grid item xs={12} md={4} xl={3}>
                        <ContentCard title='Mixnode Status'>
                            <TwoColSmallTable
                                    icons
                                    keys={['Identity Key', 'Identity Key', 'Identity Key']}
                                    values={[1789, 1789, 1789]}
                                />
                        </ContentCard>
                    </Grid>
                    <Grid item xs={12} md={8} xl={6}>    
                        <WorldMap title='Location'/>
                    </Grid>
                </Grid>
            </Box>
        </>
    )
}
