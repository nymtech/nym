import * as React from 'react';
import {
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material';
import { Box } from '@mui/system';
import { styled, Theme, useTheme } from '@mui/material/styles';
import Tooltip, { tooltipClasses, TooltipProps } from '@mui/material/Tooltip';
import LinearProgress from '@mui/material/LinearProgress';
import { cellStyles } from '../Universal-DataGrid';
import { InfoSVG } from '../../icons/InfoSVG';
import { DelegatorsInfoRowWithIndex, RowsType } from './types';
import { ColumnsType, UniversalTableProps } from '../DetailTable';

const tooltipBackGroundColor = '#A0AED1';

const formatCellValues = (val: RowsType, field: string, theme: Theme) => {
  if (val.visualProgressValue) {
    const percentageColor = val.visualProgressValue > 100 ? 'warning' : 'inherit';
    const percentageToDisplay = val.visualProgressValue > 100 ? 100 : val.visualProgressValue;

    return (
      <Box sx={{ display: 'flex', alignItems: 'center' }} id="field">
        <Typography sx={{ mr: 1, fontWeight: '600', fontSize: '12px', color: 'secondary' }}>{val.value}</Typography>
        <LinearProgress
          variant="determinate"
          value={percentageToDisplay}
          color={percentageColor}
          sx={{ width: '100px', borderRadius: '5px', backgroundColor: theme.palette.nym.networkExplorer.nav.text }}
        />
      </Box>
    );
  }
  return (
    <Typography sx={{ mr: 1, fontWeight: '600', fontSize: '12px' }} id={field}>
      {val.value}
    </Typography>
  );
};

export const DelegatorsInfoTable: React.FC<{
  tableName: string;
  columnsData: ColumnsType[];
  rows: DelegatorsInfoRowWithIndex[];
}> = ({ tableName, columnsData, rows }: UniversalTableProps) => {
  const theme = useTheme();

  const CustomTooltip = styled(({ className, ...props }: TooltipProps) => (
    <Tooltip {...props} classes={{ popper: className }} />
  ))({
    [`& .${tooltipClasses.tooltip}`]: {
      maxWidth: 230,
      background: tooltipBackGroundColor,
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
                    <Box sx={{ mr: 0.5, display: 'flex', alignItems: 'center' }}>
                      <CustomTooltip
                        title={tooltipInfo}
                        id={field}
                        placement="top-start"
                        sx={{
                          '& .MuiTooltip-arrow': {
                            color: '#A0AED1',
                          },
                        }}
                        arrow
                      >
                        <IconButton disableFocusRipple disableRipple>
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
                    padding: 2,
                    width: 200,
                    fontSize: 12,
                    fontWeight: 600,
                    color:
                      eachRow[columnsData[index].field]?.visualProgressValue > 100
                        ? theme.palette.warning.main
                        : theme.palette.nym.wallet.fee,
                  }}
                  data-testid={`${_.title.replace(/ /g, '-')}-value`}
                >
                  {formatCellValues(eachRow[columnsData[index].field], columnsData[index].field, theme)}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );
};
