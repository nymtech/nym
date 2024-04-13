import * as React from 'react'
import {
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material'
import { Box } from '@mui/system'
import { useTheme } from '@mui/material/styles'
import { Tooltip } from '@nymproject/react/tooltip/Tooltip.js'
import { EconomicsRowsType, EconomicsInfoRowWithIndex } from './types'
import { UniversalTableProps } from '@/app/components/DetailTable'
import { textColour } from '@/app/utils'

const formatCellValues = (value: EconomicsRowsType, field: string) => (
  <Box sx={{ display: 'flex', alignItems: 'center' }} id="field">
    <Typography sx={{ mr: 1, fontWeight: '600', fontSize: '12px' }} id={field}>
      {value.value}
    </Typography>
  </Box>
)

export const DelegatorsInfoTable: FCWithChildren<
  UniversalTableProps<EconomicsInfoRowWithIndex>
> = ({ tableName, columnsData, rows }) => {
  const theme = useTheme()

  return (
    <TableContainer component={Paper}>
      <Table sx={{ minWidth: 650 }} aria-label={tableName}>
        <TableHead>
          <TableRow>
            {columnsData?.map(({ field, title, tooltipInfo, width }) => (
              <TableCell
                key={field}
                sx={{ fontSize: 14, fontWeight: 600, width }}
              >
                <Box sx={{ display: 'flex', alignItems: 'center' }}>
                  {tooltipInfo && (
                    <Tooltip
                      title={tooltipInfo}
                      id={field}
                      placement="top-start"
                      textColor={
                        theme.palette.nym.networkExplorer.tooltip.color
                      }
                      bgColor={
                        theme.palette.nym.networkExplorer.tooltip.background
                      }
                      maxWidth={230}
                      arrow
                    />
                  )}
                  {title}
                </Box>
              </TableCell>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          {rows?.map((eachRow) => (
            <TableRow
              key={eachRow.id}
              sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
            >
              {columnsData?.map((_, index: number) => {
                const { field } = columnsData[index]
                const value: EconomicsRowsType = (eachRow as any)[field]
                return (
                  <TableCell
                    key={_.title}
                    sx={{
                      color: textColour(value, field, theme),
                    }}
                    data-testid={`${_.title.replace(/ /g, '-')}-value`}
                  >
                    {formatCellValues(value, columnsData[index].field)}
                  </TableCell>
                )
              })}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  )
}
