import * as React from 'react';
import { Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { Tooltip } from '@nymproject/react/tooltip/Tooltip';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { Box } from '@mui/system';
import { cellStyles } from './Universal-DataGrid';
import { currencyToString } from '../utils/currency';
import { GatewayEnridedRowType } from './Gateways';
import { MixnodeRowType } from './MixNodes';

export type ColumnsType = {
  field: string;
  title: string;
  headerAlign: string;
  flex?: number;
  width?: number;
  tooltipInfo?: string;
};

export interface UniversalTableProps<T = any> {
  tableName: string;
  columnsData: ColumnsType[];
  rows: T[];
}

function formatCellValues(val: string | number, field: string) {
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
  return val;
}

export const DetailTable: React.FC<{
  tableName: string;
  columnsData: ColumnsType[];
  rows: MixnodeRowType[] | GatewayEnridedRowType[];
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
                    <Box sx={{ display: 'flex', alignItems: 'center' }}>
                      <Tooltip
                        title={tooltipInfo}
                        id={field}
                        placement="top-start"
                        textColor={theme.palette.nym.networkExplorer.tooltip.color}
                        bgColor={theme.palette.nym.networkExplorer.tooltip.background}
                        maxWidth={230}
                        arrow
                      />
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
                    padding: 2,
                    width: 200,
                    fontSize: 14,
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
