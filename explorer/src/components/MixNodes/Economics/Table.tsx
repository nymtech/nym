import * as React from 'react';
import { Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Typography } from '@mui/material';
import { Box } from '@mui/system';
import { useTheme, Theme } from '@mui/material/styles';
import { Tooltip } from '@nymproject/react/tooltip/Tooltip';
import { EconomicsRowsType, EconomicsInfoRowWithIndex } from './types';
import { EconomicsProgress } from './EconomicsProgress';
import { cellStyles } from '../../Universal-DataGrid';
import { UniversalTableProps } from '../../DetailTable';
import { useIsMobile } from '../../../hooks/useIsMobile';

const threshold = 100;

const textColour = (value: EconomicsRowsType, field: string, theme: Theme) => {
  const progressBarValue = value?.progressBarValue || 0;
  const fieldValue = value.value;

  if (progressBarValue > 100) {
    return theme.palette.warning.main;
  }
  if (field === 'selectionChance') {
    switch (fieldValue) {
      case 'High':
      case 'Very High':
        return theme.palette.nym.networkExplorer.selectionChance.overModerate;
      case 'Moderate':
        return theme.palette.nym.networkExplorer.selectionChance.moderate;
      case 'Low':
      case 'Very Low':
        return theme.palette.nym.networkExplorer.selectionChance.underModerate;
      default:
        return theme.palette.nym.wallet.fee;
    }
  }
  return theme.palette.nym.wallet.fee;
};

const formatCellValues = (value: EconomicsRowsType, field: string) => {
  const isTablet = useIsMobile('lg');
  if (value.progressBarValue) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', flexDirection: isTablet ? 'column' : 'row' }} id="field">
        <Typography
          sx={{
            mr: isTablet ? 0 : 1,
            mb: isTablet ? 1 : 0,
            fontWeight: '600',
            fontSize: '12px',
          }}
          id={field}
        >
          {value.value}
        </Typography>
        <EconomicsProgress threshold={threshold} value={value.progressBarValue} />
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

export const DelegatorsInfoTable: React.FC<UniversalTableProps<EconomicsInfoRowWithIndex>> = ({
  tableName,
  columnsData,
  rows,
}) => {
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
          {rows?.map((eachRow) => (
            <TableRow key={eachRow.id} sx={{ '&:last-child td, &:last-child th': { border: 0 } }}>
              {columnsData?.map((_, index: number) => {
                const { field } = columnsData[index];
                const value: EconomicsRowsType = (eachRow as any)[field];

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
                      color: textColour(value, field, theme),
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
