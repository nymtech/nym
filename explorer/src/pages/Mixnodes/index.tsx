import React from 'react';
import { Box, Grid, IconButton, Typography } from '@mui/material';
import { MixnodesTable } from '../../components/Table';
import { MainContext } from 'src/context/main';
import { TableHeadingsType } from "../../typeDefs/tables";

const tableHeadings: TableHeadingsType = [
    {
        id: 'owner',
        numeric: false,
        disablePadding: true,
        label: 'Owner',
    },
    {
        id: 'id_key',
        numeric: true,
        disablePadding: false,
        label: 'Identity Key',
    },
    {
        id: 'bond',
        numeric: true,
        disablePadding: false,
        label: 'Bond)',
    },
    {
        id: 'ip_port',
        numeric: true,
        disablePadding: false,
        label: 'IP:Port',
    },
    {
        id: 'location',
        numeric: true,
        disablePadding: false,
        label: 'Location',
    },
    {
        id: 'layer',
        numeric: true,
        disablePadding: false,
        label: 'Layer',
    },
]

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
                        <MixnodesTable headings={tableHeadings} mixnodes={mixnodes} />
                    </Grid>
                </Grid>
            </Box>
        </>
    )
}
