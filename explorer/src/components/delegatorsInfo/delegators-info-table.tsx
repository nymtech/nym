import * as React from 'react';
import { Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Tooltip, IconButton } from '@mui/material';
import { CopyToClipboard } from '@nymproject/react';
import { Box } from '@mui/system';
import { useTheme } from '@mui/material/styles';
import { cellStyles } from '../Universal-DataGrid';
import { currencyToString } from '../../utils/currency';
import { InfoSVG } from '../../icons/InfoSVG';

export type ColumnsType = {
    field: string;
    title: string;
    headerAlign: string;
    flex?: number;
    width?: number;
    tooltipInfo?: boolean;
};

export type RowsType = {
    value: string;
    visualProgressValue?: number;
};

export interface UniversalTableProps {
    tableName: string;
    columnsData: ColumnsType[];
    rows: any[];
}

function formatCellValues(val: RowsType, field: string) {
    console.log('val', val, 'field', field);
    if (val.visualProgressValue) {

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
                                            <Tooltip title={tooltipInfo}>
                                                <IconButton>
                                                    <InfoSVG />
                                                </IconButton>
                                            </Tooltip>
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
