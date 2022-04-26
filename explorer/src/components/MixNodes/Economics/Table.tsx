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
import { styled, useTheme } from '@mui/material/styles';
import Tooltip, { tooltipClasses, TooltipProps } from '@mui/material/Tooltip';
import { RowsType, DelegatorsInfoRowWithIndex } from './types';
import { EconomicsProgress } from './EconomicsProgress';
import { cellStyles } from '../../Universal-DataGrid';
import { InfoSVG } from '../../../icons/InfoSVG';
import { UniversalTableProps } from '../../DetailTable';

const tooltipBackGroundColor = '#A0AED1';
const threshold = 100;

const formatCellValues = (value: RowsType, field: string) => {
  if (value.displayEconProgress && Number.isInteger(value?.value)) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center' }} id="field">
        <Typography
          sx={{
            mr: 1,
            fontWeight: '600',
            fontSize: '12px',
          }}
          id={field}
        >
          {`${value?.value?.toFixed(2)} %`}
        </Typography>
        <EconomicsProgress threshold={threshold} value={value?.value} />
      </Box>
    );
  }
  return (
    <Box sx={{ display: 'flex', alignItems: 'center' }} id="field">
      <Typography sx={{ mr: 1, fontWeight: '600', fontSize: '12px' }} id={field}>
        {value.value}
      </Typography>
    </Box>
  );
};

export const DelegatorsInfoTable: React.FC<UniversalTableProps<DelegatorsInfoRowWithIndex>> = ({
  tableName,
  columnsData,
  rows,
}) => {
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
              {columnsData?.map((_, index: number) => {
                const { field } = columnsData[index];
                const value = (eachRow as any)[field];
                return (
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
                      color: value?.percentaje > 100 ? theme.palette.warning.main : theme.palette.nym.wallet.fee,
                    }}
                    data-testid={`${_.title.replace(/ /g, '-')}-value`}
                  >
                    {formatCellValues(value, columnsData[index].field)}
                  </TableCell>
                );
              })}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );
};
