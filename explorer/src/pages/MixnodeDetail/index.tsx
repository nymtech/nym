import * as React from 'react';
import { Box, Grid, Typography } from '@mui/material';
import { MixnodesTable } from '../../components/MixnodesTable';
import { MainContext } from 'src/context/main';
import { useParams } from 'react-router-dom';
import { ContentCard } from 'src/components/ContentCard';
import { BondBreakdownTable } from 'src/components/BondBreakdown';
import { TwoColSmallTable } from 'src/components/TwoColSmallTable';

export const PageMixnodeDetail: React.FC = () => {
    const { fetchMixnodeById, mixnodeDetailInfo } = React.useContext(MainContext);
    let { id }: any = useParams();

    React.useEffect(() => {
        const hasNoDetail = id && !mixnodeDetailInfo;
        const hasIncorrectDetail = id && mixnodeDetailInfo?.data && mixnodeDetailInfo?.data[0].mix_node.identity_key !== id;
        if (hasNoDetail || hasIncorrectDetail) {
            fetchMixnodeById(id)
        }
    }, [id, fetchMixnodeById]);

    return (
        <>
            <Box component='main' sx={{ flexGrow: 1 }}>
                <Grid container spacing={2}>
                    <Grid item xs={12}>
                        <Typography>
                            Mixnode Detail
                        </Typography>
                    </Grid>
                    <Grid item xs={12}>
                        {mixnodeDetailInfo && (
                            <MixnodesTable mixnodes={mixnodeDetailInfo} />
                        )}
                    </Grid>

                    <Grid item xs={12}>
                        <ContentCard title='Bond Breakdown'>
                            <BondBreakdownTable />
                        </ContentCard>
                    </Grid>
                    <Grid item xs={12} md={4}>
                        <ContentCard title='Mixnode Stats'>
                            
                            <TwoColSmallTable
                                title='Since startup'
                                keys={['Received', 'Sent', 'Explicitly dropped']}
                                values={[1789, 1789, 1789]}
                            />
                            
                            <TwoColSmallTable
                                title='Since startup'
                                keys={['Received', 'Sent', 'Explicitly dropped']}
                                values={[1789, 1789, 1789]}
                            />
                        </ContentCard>
                    </Grid>


                    <Grid item xs={12} md={8}>
                        <ContentCard title='uptine story'>
                            <p>I am the uptime story</p>
                        </ContentCard>
                    </Grid>


                    <Grid item xs={12} md={4}>
                        <ContentCard title='Mixnode Status'>

                            <TwoColSmallTable
                                    icons
                                    keys={['Identity Key', 'Identity Key', 'Identity Key']}
                                    values={[1789, 1789, 1789]}
                                />
                        </ContentCard>
                    </Grid>
                </Grid>
            </Box>
        </>
    )
}
