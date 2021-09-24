import React from 'react';
import { Box, Grid, Typography } from '@mui/material';
import { MixnodesTable } from '../../components/MixnodesTable';
import { MainContext } from 'src/context/main';

export const PageMixnodes: React.FC = () => {
    const { mixnodes } = React.useContext(MainContext);
    return (
        <>
            <Box component='main' sx={{ flexGrow: 1 }}>
                <Grid container spacing={0}>
                    <Grid item xs={12}>
                        <Typography sx={{ marginLeft: 3 }}>
                            Mixnodes
                        </Typography>
                    </Grid>
                    <Grid item xs={11}>
                        <MixnodesTable mixnodes={mixnodes} />
                    </Grid>
                </Grid>
            </Box>
        </>
    )
}
