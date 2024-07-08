import React from 'react';
import {
  Stack,
  SxProps,
  Table,
  TableBody,
  TableCell,
  TableCellProps,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material';
import { InfoTooltip } from '../InfoToolTip';

export type Header = { header?: string; id: string; tooltipText?: string; sx?: SxProps };
export type Cell = { cell: string | React.ReactNode; id: string; align?: TableCellProps['align']; sx?: SxProps };

export interface TableProps {
  headers: Header[];
  cells: Cell[];
}

export const NodeTable = ({ headers, cells }: TableProps) => (
  <TableContainer>
    <Table aria-label="node-table">
      <TableHead>
        <TableRow>
          {headers.map(({ header, id, tooltipText }) => (
            <TableCell key={id}>
              <Stack direction="row" alignItems="center" gap={1}>
                {tooltipText && <InfoTooltip title={tooltipText} />}
                <Typography>{header}</Typography>
              </Stack>
            </TableCell>
          ))}
        </TableRow>
      </TableHead>
      <TableBody>
        <TableRow key="node-data">
          {cells.map(({ cell, id, align }) => (
            <TableCell key={id} align={align} sx={{ textTransform: 'uppercase' }}>
              {cell}
            </TableCell>
          ))}
        </TableRow>
      </TableBody>
    </Table>
  </TableContainer>
);
