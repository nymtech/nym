import * as React from 'react';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import CheckCircleSharpIcon from '@mui/icons-material/CheckCircleSharp';
import ErrorIcon from '@mui/icons-material/Error';
import { Typography } from '@mui/material';

interface TableProps {
    title?: string
    icons?: boolean[]
    keys: string[]
    values: number[]
    marginBottom?: boolean
}

export function TwoColSmallTable({ title, icons, keys, values, marginBottom }: TableProps) {
        return (
            <>
                {title && (
                    <Typography sx={{ marginTop: 2 }}>
                        {title}
                    </Typography>
                )}
                <TableContainer component={Paper} sx={ marginBottom ? { marginBottom: 4, marginTop: 2 } : { marginTop: 2 }}>
                    <Table aria-label='two col small table'>
                        <TableBody>
                            {keys.map((each: string, i: number) => {
                                return (
                                    <TableRow key={i}>
                                        { icons && (
                                            <TableCell>
                                                { icons[i] ? <CheckCircleSharpIcon /> : <ErrorIcon /> } 
                                            </TableCell>
                                            )}
                                        <TableCell>{each}</TableCell>
                                        <TableCell align='right'>{values[i]}</TableCell>
                                    </TableRow>
                                )
                            })}
                        </TableBody>
                    </Table>
                </TableContainer>
            </>
        );
}
