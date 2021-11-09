import * as React from 'react';
import {
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
} from '@mui/material';
import { printableCoin } from '@nymproject/nym-validator-client';
import { cellStyles } from './Universal-DataGrid';

export interface UniversalTableProps {
  tableName: string;
  columnsData: {
    field: string;
    title: string;
    headerAlign: string;
    flex: number;
  }[];
  rows: any[];
}

function formatCellValues(val: string | number, field: string) {
  if (field === 'bond') {
    return printableCoin({ amount: val.toString(), denom: 'unpunk' });
  }
  return val;
}

export const UniversalTable = ({
  tableName,
  columnsData,
  rows,
}: UniversalTableProps) => (
  <TableContainer component={Paper}>
    <Table sx={{ minWidth: 650 }} aria-label={tableName}>
      <TableHead>
        <TableRow>
          {columnsData?.map(({ field, title, flex }) => (
            <TableCell key={field} sx={{ fontWeight: 'bold', flex }}>
              {title}
            </TableCell>
          ))}
        </TableRow>
      </TableHead>
      <TableBody>
        {rows.map((eachRow) => (
          <TableRow
            key={eachRow.owner}
            sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
          >
            {columnsData?.map((_, index) => (
              <TableCell
                component="th"
                scope="row"
                variant="body"
                sx={{
                  ...cellStyles,
                  padding: 2,
                  width: 200,
                }}
              >
                {formatCellValues(
                  eachRow[columnsData[index].field],
                  columnsData[index].field,
                )}
              </TableCell>
            ))}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  </TableContainer>
);
