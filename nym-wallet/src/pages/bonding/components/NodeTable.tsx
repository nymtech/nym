import React from 'react';
import {
  Box,
  Stack,
  SxProps,
  Table,
  TableBody,
  TableCell as MUITableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from '@mui/material';
import { InfoOutlined } from '@mui/icons-material';

export interface TableCell {
  children: React.ReactNode;
  color?: string;
  align?: 'center' | 'inherit' | 'justify' | 'left' | 'right';
  size?: 'small' | 'medium';
  sx?: SxProps;
}

export type TableHeader = TableCell & { tooltip?: React.ReactNode };

const CellHeader = ({ children, tooltip, sx, size, align, color }: TableHeader) => (
  <MUITableCell sx={{ py: 1.2, color, ...sx }} size={size} align={align}>
    {tooltip ? (
      <Tooltip title={tooltip} arrow placement="top-start">
        <Stack direction="row" alignItems="center" fontSize="0.8rem">
          <InfoOutlined fontSize="inherit" sx={{ mr: 0.5 }} />
          <Typography>{children}</Typography>
        </Stack>
      </Tooltip>
    ) : (
      <Typography>{children}</Typography>
    )}
  </MUITableCell>
);

const CellValue = ({ children, align, size, color, sx }: TableCell) => (
  <MUITableCell component="th" scope="row" sx={{ py: 1, color, ...sx }} align={align} size={size}>
    {children}
  </MUITableCell>
);

export type Header = Omit<TableHeader, 'children'> & { header?: React.ReactNode; id: string };
export type Cell = Omit<TableCell, 'children'> & { cell: React.ReactNode; id: string };

export interface TableProps {
  headers: Header[];
  cells: Cell[];
}

const NodeTable = ({ headers, cells }: TableProps) => (
  <TableContainer component={Box}>
    <Table sx={{ minWidth: 650 }} aria-label="node-table">
      <TableHead>
        <TableRow>
          {headers.map(({ header, id, tooltip, sx }) => (
            <CellHeader tooltip={tooltip} key={id} sx={sx}>
              {header}
            </CellHeader>
          ))}
        </TableRow>
      </TableHead>
      <TableBody>
        <TableRow key="node-data">
          {cells.map(({ cell, id, align, size, color, sx }) => (
            <CellValue align={align} size={size} key={id} sx={sx} color={color}>
              {cell}
            </CellValue>
          ))}
        </TableRow>
      </TableBody>
    </Table>
  </TableContainer>
);

export default NodeTable;
