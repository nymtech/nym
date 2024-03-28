import * as React from 'react';
import {
  Link,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TableCellProps,
} from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { Tooltip } from '@nymproject/react/tooltip/Tooltip';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { Box } from '@mui/system';
import { unymToNym } from '../utils/currency';
import { GatewayEnrichedRowType } from './Gateways';
import { MixnodeRowType } from './MixNodes';
import { StakeSaturationProgressBar } from './MixNodes/Economics/StakeSaturationProgressBar';

export type ColumnsType = {
  field: string;
  title: string;
  headerAlign?: TableCellProps['align'];
  width?: string | number;
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
    return unymToNym(val, 6);
  }

  if (field === 'owner') {
    return (
      <Link underline="none" color="inherit" target="_blank" href={`https://mixnet.explorers.guru/account/${val}`}>
        {val}
      </Link>
    );
  }

  if (field === 'stake_saturation') {
    return <StakeSaturationProgressBar value={Number(val)} threshold={100} />;
  }

  return val;
}

export const DetailTable: FCWithChildren<{
  tableName: string;
  columnsData: ColumnsType[];
  rows: MixnodeRowType[] | GatewayEnrichedRowType[];
}> = ({ tableName, columnsData, rows }: UniversalTableProps) => {
  const theme = useTheme();
  return (
    <TableContainer component={Paper}>
      <Table sx={{ minWidth: 1080 }} aria-label={tableName}>
        <TableHead>
          <TableRow>
            {columnsData?.map(({ field, title, width, tooltipInfo }) => (
              <TableCell key={field} sx={{ fontSize: 14, fontWeight: 600, width }}>
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
              {columnsData?.map((data, index) => (
                <TableCell
                  key={data.title}
                  component="th"
                  scope="row"
                  variant="body"
                  sx={{
                    padding: 2,
                    width: 200,
                    fontSize: 14,
                  }}
                  data-testid={`${data.title.replace(/ /g, '-')}-value`}
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
