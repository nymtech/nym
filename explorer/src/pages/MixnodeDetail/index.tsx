import * as React from 'react';
import { Box, Grid, Typography } from '@mui/material';
import { MixnodesTable } from '../../components/MixnodesTable';
import { MainContext } from 'src/context/main';
import { useParams } from 'react-router-dom';
import { ContentCard } from 'src/components/ContentCard';
import { BondBreakdownTable } from 'src/components/BondBreakdown';

export const PageMixnodeDetail: React.FC = () => {
    const { fetchMixnodeById, mixnodeDetailInfo } = React.useContext(MainContext);
    let { id }: any = useParams();

    React.useEffect(() => {
        // if ID is in URL, there's no selected mixnode at all (null)
        // or the mixnode that's in there is diff to ID, then fetch/filter down to the new
        // one.
        if(id && !mixnodeDetailInfo || mixnodeDetailInfo?.mix_node.identity_key !== id) {
            console.log("fetching a new mixnodeDetailInfo", id);
            fetchMixnodeById(id)
        }
        console.log("mixnodeDetailInfo is back ", mixnodeDetailInfo);
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
                            <MixnodesTable
                                mixnodes={{
                                    data: [ mixnodeDetailInfo ],
                                    isLoading: false
                                }}
                            />
                        )}
                    </Grid>

                    <Grid item xs={12}>
                        <ContentCard title='Bond Breakdown'>
                            <BondBreakdownTable />
                        </ContentCard>
                    </Grid>
                    <Grid item xs={12} md={6}>
                        <ContentCard title='Mixnode Stats'>
                            <p>I am the mixnode stats</p>
                        </ContentCard>
                    </Grid>
                    <Grid item xs={12} md={6}>
                        <ContentCard title='uptine story'>
                            <p>I am the uptime story</p>
                        </ContentCard>
                    </Grid>
                </Grid>
            </Box>
        </>
    )
}
