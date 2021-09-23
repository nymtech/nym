import * as React from 'react';
import { Box, Grid, IconButton, Typography } from '@mui/material';
import { MixnodesTable } from '../../components/Table';
import { MainContext } from 'src/context/main';
import { TableHeadingsType } from "../../typeDefs/tables";
import { MixNodeResponseItem } from 'src/typeDefs/node-status-api-client';
import { useLocation, useParams } from 'react-router-dom';

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

export const PageMixnodeInfo: React.FC = () => {
    const { mixnodes } = React.useContext(MainContext);
    let location = useLocation();
    let { id }: any = useParams();

    const [nodeInfo, setNodeInfo] = React.useState<MixNodeResponseItem | null>(null);

    React.useEffect(() => {
        // @ts-ignore
        const thisNode: MixNodeResponseItem = mixnodes && mixnodes?.data?.filter((eachMixnode: MixNodeResponseItem) => {
            return eachMixnode.mix_node.identity_key === id
        })[0];
        setNodeInfo(thisNode)
    }, [mixnodes])
    return (
        <>
            <Box component='main' sx={{ flexGrow: 1 }}>
                <Grid container spacing={0}>
                    <Grid item xs={12}>
                        <Typography sx={{ marginLeft: 3 }}>
                            Mixnode Info
                        </Typography>
                    </Grid>

                    <Grid item xs={12}>
                        {/* add in the same headings as before
                        add in the one row of data for this mixnode */}
                        <MixnodesTable />
                    </Grid>
                </Grid>
            </Box>
        </>
    )
}
