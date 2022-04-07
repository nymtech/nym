import * as React from 'react';
import { Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, IconButton } from '@mui/material';
import { CopyToClipboard } from '@nymproject/react';
import { Box } from '@mui/system';
import { useTheme, styled } from '@mui/material/styles';
import Tooltip, { TooltipProps, tooltipClasses } from '@mui/material/Tooltip';
import { cellStyles } from '../Universal-DataGrid';
import { currencyToString } from '../../utils/currency';
import { InfoSVG } from '../../icons/InfoSVG';
import { ColumnsType, RowsType, UniversalTableProps } from './types';



const formatCellValues = (val: RowsType, field: string) => {

    if (val.visualProgressValue) {
        console.log('val', val, 'field', field);
    }
    if (field === 'identity_key' && typeof val === 'string') {
        return (
            <Box display="flex" justifyContent="flex-end">
                <CopyToClipboard
                    sx={{ mr: 1, mt: 0.5, fontSize: '18px' }}
                    value={val}
                    tooltip={`Copy identity key ${val} to clipboard`}
                />
                <span>{val}</span>
            </Box>
        );
    }
    if (field === 'bond') {
        return currencyToString(val.toString());
    }
    return val.value;
}

export const DelegatorsInfoTable: React.FC<{
    tableName: string;
    columnsData: ColumnsType[];
    rows: [];
}> = ({ tableName, columnsData, rows }: UniversalTableProps) => {
    const theme = useTheme();

    const CustomTooltip = styled(({ className, ...props }: TooltipProps) => (
        <Tooltip {...props} classes={{ popper: className }} />
    ))({
        [`& .${tooltipClasses.tooltip}`]: {
            maxWidth: 230,
            background: '#A0AED1',
            color: theme.palette.nym.networkExplorer.nav.hover,
        },
    });
    
    return (
        <TableContainer component={Paper}>
            <Table sx={{ minWidth: 650 }} aria-label={tableName}>
                <TableHead>
                    <TableRow>
                        {columnsData?.map(({ field, title, flex, tooltipInfo }) => (
                            <TableCell key={field} sx={{ fontSize: 14, fontWeight: 600, flex }}>
                                <Box sx={{ display: 'flex', alignItems: 'center' }}>
                                    {tooltipInfo && (
                                        <Box sx={{ mr: .5, display: 'flex', alignItems: 'center' }}>
                                            <CustomTooltip
                                                title={tooltipInfo}
                                                id={field}
                                                placement='top-start'
                                                sx={{
                                                    '& .MuiTooltip-arrow': {
                                                        color: '#A0AED1',
                                                    },
                                                }}
                                                arrow
                                            >
                                                <IconButton>
                                                    <InfoSVG />
                                                </IconButton>
                                            </CustomTooltip>
                                        </Box>
                                    )}
                                    {title}
                                </Box>
                            </TableCell>
                        ))}
                    </TableRow>
                </TableHead>
                <TableBody>
                    {rows.map((eachRow) => (
                        <TableRow key={eachRow.id} sx={{ '&:last-child td, &:last-child th': { border: 0 } }}>
                            {columnsData?.map((_, index) => (
                                <TableCell
                                    key={_.title}
                                    component="th"
                                    scope="row"
                                    variant="body"
                                    sx={{
                                        ...cellStyles,
                                        color: theme.palette.nym.wallet.fee,
                                        padding: 2,
                                        width: 200,
                                        fontSize: 12,
                                        fontWeight: 600,
                                    }}
                                    data-testid={`${_.title.replace(/ /g, '-')}-value`}
                                >
                                    {formatCellValues(eachRow[columnsData[index].field], columnsData[index].field)}
                                </TableCell>
                            ))}
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </TableContainer>
    );
};
