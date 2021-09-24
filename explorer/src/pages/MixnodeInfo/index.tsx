import * as React from 'react';
import { Box, Grid, IconButton, Typography } from '@mui/material';
import { MixnodesTable } from '../../components/Table';
import { MainContext } from 'src/context/main';
import { TableHeadingsType } from "../../typeDefs/tables";
import { MixNodeResponseItem } from 'src/typeDefs/explorer-api';
import { useParams } from 'react-router-dom';
import { MixNodeResponse } from 'src/typeDefs/explorer-api';

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

type SelectedNodeType = {
    isLoading: boolean,
    data?: MixNodeResponse,
    error?: Error
}

export const PageMixnodeInfo: React.FC = () => {
    const { mixnodes } = React.useContext(MainContext);
    let { id }: any = useParams();

    const [selectedNodeInfo, setSelectedNodeInfo] = React.useState<SelectedNodeType>();

    React.useEffect(() => {
        // @ts-ignore
        const data: MixNodeResponse = mixnodes && mixnodes?.data?.filter((eachMixnode: MixNodeResponseItem) => {
            return eachMixnode.mix_node.identity_key === id
        });
        setSelectedNodeInfo({ data, isLoading: false })
    }, [mixnodes])
    return (
        <>
            <Box component='main' sx={{ flexGrow: 1 }}>
                <Grid container spacing={0}>
                    <Grid item xs={12}>
                        <Typography sx={{ marginLeft: 3 }}>
                            Mixnode Info
                        </Typography>
                        <MixnodesTable headings={tableHeadings} mixnodes={selectedNodeInfo} />
                    </Grid>
                </Grid>
            </Box>
        </>
    )
}
