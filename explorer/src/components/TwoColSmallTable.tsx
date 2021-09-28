import * as React from 'react';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
// import { TableHeadingsType, TableHeading } from 'src/typeDefs/tables';
// import { MixNodeResponseItem, MixNodeResponse, ApiState } from 'src/typeDefs/explorer-api';
// import { Link } from 'react-router-dom';
import CheckCircleSharpIcon from '@mui/icons-material/CheckCircleSharp';
import { Typography } from '@mui/material';

interface TableProps {
    title?: string
    icons?: boolean
    keys: string[]
    values: number[]
}

export function TwoColSmallTable({ title, icons, keys, values }: TableProps) {
        return (
            <>
                {title && (
                    <Typography sx={{ marginTop: 2 }}>
                        {title}
                    </Typography>
                )}
                <TableContainer component={Paper} sx={{ marginBottom: 4, marginTop: 2 }}>
                    <Table aria-label='two col small table'>
                        <TableBody>
                            <TableRow>
                                { icons && <TableCell sx={{ paddingLeft: 0 }}><CheckCircleSharpIcon /></TableCell>}
                                <TableCell sx={{ paddingLeft: 0 }}>Received</TableCell>
                                <TableCell sx={{ paddingLeft: 0 }}>1789</TableCell>
                            </TableRow>
                            <TableRow>
                                { icons && <TableCell sx={{ paddingLeft: 0 }}><CheckCircleSharpIcon /></TableCell>}
                                <TableCell sx={{ paddingLeft: 0 }}>Sent</TableCell>
                                <TableCell sx={{ paddingLeft: 0 }}>1789</TableCell>
                            </TableRow>
                            <TableRow>
                                { icons && <TableCell sx={{ paddingLeft: 0 }}><CheckCircleSharpIcon /></TableCell>}
                                <TableCell sx={{ paddingLeft: 0 }}>Explicitly dropped</TableCell>
                                <TableCell sx={{ paddingLeft: 0 }}>1789</TableCell>
                            </TableRow>
                        </TableBody>
                    </Table>
                </TableContainer>
            </>
        );
}
