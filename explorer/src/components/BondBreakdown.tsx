import * as React from 'react';
import Table from '@mui/material/Table';
import { useMediaQuery, useTheme } from '@mui/material';
import TableBody from '@mui/material/TableBody';
import { useParams } from 'react-router-dom';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import { MainContext } from 'src/context/main';

export function BondBreakdownTable() {
    const { id }: any = useParams();
    const { mixnodeDetailInfo, delegations } = React.useContext(MainContext);
    const [bonds, setBonds] = React.useState({
        delegations: 0,
        pledges: 0,
        bondsTotal: 0,
        denom: 'PUNK',
    })
    const theme = useTheme();
    const matches = useMediaQuery(theme.breakpoints.down("sm"));

    React.useEffect(() => {
        if (mixnodeDetailInfo && mixnodeDetailInfo.data?.length) {
            const thisMixnode = mixnodeDetailInfo?.data[0];
            const delegations = Number(thisMixnode.total_delegation.amount);
            const pledges = Number(thisMixnode.bond_amount.amount);
            const bondsTotal = delegations + pledges;
            setBonds({
                delegations,
                pledges,
                bondsTotal,
                denom: thisMixnode.total_delegation.denom.toUpperCase()
            });
        }
    }, [mixnodeDetailInfo]);

    return (
        <>
            <TableContainer component={Paper}>
                <Table sx={{ minWidth: 650 }} aria-label='bond breakdown totals'>
                    <TableBody>
                        <TableRow sx={matches ? { minWidth: '70vw' } : null}>
                            <TableCell
                                sx={{
                                    fontWeight: 'bold',
                                    width: matches ? '90px' : 'auto',
                                }}
                                align='left'
                            >
                                Bond total
                            </TableCell>
                            <TableCell align='left'>
                                {bonds.bondsTotal}{bonds.denom}
                            </TableCell>
                        </TableRow>
                        <TableRow>
                            <TableCell
                                sx={{
                                    width: matches ? '90px' : 'auto',
                                }}
                                align='left'
                            >
                                Pledge total
                            </TableCell>
                            <TableCell align='left'>
                                {bonds.pledges}{bonds.denom}
                            </TableCell>
                        </TableRow>
                        <TableRow>
                            <TableCell
                                sx={{
                                    width: matches ? '90px' : 'auto',
                                }}
                                align='left'
                            >
                                Delegation total
                            </TableCell>
                            <TableCell align='left'>
                                {bonds.delegations}{bonds.denom}
                            </TableCell>
                        </TableRow>
                    </TableBody>
                </Table>

                {delegations?.data !== undefined && delegations?.data[0] && (
                    <Table sx={{ minWidth: 650 }} aria-label='delegation totals'>
                        <TableHead>
                            <TableRow>
                                <TableCell sx={{ fontWeight: 'bold' }} align='left'>Owner</TableCell>
                                <TableCell sx={{ fontWeight: 'bold' }} align='left'>Stake</TableCell>
                                <TableCell sx={{ fontWeight: 'bold' }} align='left'>Share from bond</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {delegations.data.map(({ owner, amount: { amount, denom }, block_height }) => {
                                return (
                                    <TableRow key={owner}>
                                        <TableCell sx={matches ? { width: 190 } : null} align='left'>{owner}</TableCell>
                                        <TableCell align='left'>{amount}{denom.toUpperCase()}</TableCell>
                                        <TableCell align='left'>400%</TableCell>
                                    </TableRow>
                                )
                            })}
                        </TableBody>
                    </Table>
                )}
            </TableContainer>
        </>
    );
}
