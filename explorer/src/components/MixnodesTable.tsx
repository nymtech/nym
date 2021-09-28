import * as React from 'react';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import { TableHeadingsType, TableHeading } from 'src/typeDefs/tables';
import { MixNodeResponse, ApiState, MixNodeResponseItem } from 'src/typeDefs/explorer-api';
import { Link } from 'react-router-dom';

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

type TableProps = {
    mixnodes: ApiState<MixNodeResponse>
}

export function MixnodesTable({ mixnodes }: TableProps) {
    if (mixnodes && mixnodes.data && mixnodes.data.length) {
        return (
            <TableContainer component={Paper}>
                <Table sx={{ minWidth: 650 }} aria-label='mixnodes table'>
                    <TableHead>
                        <TableRow>
                            {tableHeadings.map((eachHeading: TableHeading, i: number) => (
                                <TableCell sx={{ fontWeight: 'bold' }} key={eachHeading.id} align='left'>{eachHeading.label}</TableCell>
                            ))}
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {mixnodes.data.map((row: MixNodeResponseItem) => (
                            <TableRow
                                key={row.mix_node.identity_key}
                                sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
                            >
                                <TableCell component='th' scope='row' sx={{ maxWidth: 250, wordBreak: 'break-all' }}>
                                    {row.owner}
                                </TableCell>
                                <TableCell sx={{ maxWidth: 250, wordBreak: 'break-all' }} align='left'>
                                    <Link to={`/network-components/mixnodes/${row.mix_node.identity_key}`} style={{ textDecoration: 'none', color: 'white' }}>
                                        {row.mix_node.identity_key}
                                    </Link>
                                </TableCell>
                                <TableCell align='left'>{`${row.bond_amount.amount}${row.bond_amount.denom.toUpperCase()}`}</TableCell>
                                <TableCell sx={{ maxWidth: 170 }} align='left'>{row.mix_node.host}</TableCell>
                                <TableCell align='left'>{row?.location?.country_name || 'Unknown'}</TableCell>
                                <TableCell align='right'>{row.layer}</TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </TableContainer>
        );
    } else {
        return <h1>Loading...</h1>
    }
}
