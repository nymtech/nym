import * as React from 'react';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import { TableHeadingsType, TableHeading } from 'src/typeDefs/tables';
import { MixNodeResponseItem, MixNodeResponse, ApiState } from 'src/typeDefs/explorer-api';
import { Link } from 'react-router-dom';

export function BondBreakdownTable() {
    
        return (
            <TableContainer component={Paper}>
                <Table sx={{ minWidth: 650 }} aria-label='bond breakdown totals'>
                    <TableBody>
                        <TableRow>
                            <TableCell sx={{ fontWeight: 'bold' }} align='left'>Bond total</TableCell>
                            <TableCell align='left'>98676.24867PUNK</TableCell>
                        </TableRow>
                        <TableRow>
                            <TableCell align='left'>Pledge total</TableCell>
                            <TableCell align='left'>98676.24867PUNK</TableCell>
                        </TableRow>
                        <TableRow>
                            <TableCell align='left'>Delegation total</TableCell>
                            <TableCell align='left'>98676.24867PUNK</TableCell>
                        </TableRow>
                    </TableBody>
                </Table>
                <Table sx={{ minWidth: 650 }} aria-label='delegation totals'>
                    <TableHead>
                        <TableRow>
                            <TableCell sx={{ fontWeight: 'bold' }} align='left'>Owner</TableCell>
                            <TableCell sx={{ fontWeight: 'bold' }} align='left'>Stake</TableCell>
                            <TableCell sx={{ fontWeight: 'bold' }} align='left'>Share from bond</TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        <TableRow>
                            <TableCell align='left'>PUNK286492649876204989035802</TableCell>
                            <TableCell align='left'>3246</TableCell>
                            <TableCell align='left'>400%</TableCell>
                        </TableRow>
                        <TableRow>
                            <TableCell align='left'>PUNK286492649876204989035802</TableCell>
                            <TableCell align='left'>3246</TableCell>
                            <TableCell align='left'>400%</TableCell>
                        </TableRow>
                        <TableRow>
                            <TableCell align='left'>PUNK286492649876204989035802</TableCell>
                            <TableCell align='left'>3246</TableCell>
                            <TableCell align='left'>400%</TableCell>
                        </TableRow>
                    </TableBody>
                </Table>
            </TableContainer>
        );
}
