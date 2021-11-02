import * as React from 'react';
import { useTheme, CircularProgress, Typography } from '@mui/material';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import CheckCircleSharpIcon from '@mui/icons-material/CheckCircleSharp';

import ErrorIcon from '@mui/icons-material/Error';
import { MainContext } from 'src/context/main';

interface TableProps {
  title?: string;
  icons?: boolean[];
  keys: string[];
  values: number[];
  marginBottom?: boolean;
  error?: string;
  loading: boolean;
}

export const TwoColSmallTable: React.FC<TableProps> = ({
  loading,
  title,
  icons,
  keys,
  values,
  marginBottom,
  error,
}) => {
  const theme = useTheme();
  const { mode } = React.useContext(MainContext);
  return (
    <>
      {title && <Typography sx={{ marginTop: 2 }}>{title}</Typography>}

      <TableContainer
        component={Paper}
        sx={marginBottom ? { marginBottom: 4, marginTop: 2 } : { marginTop: 2 }}
      >
        <Table aria-label="two col small table">
          <TableBody>
            {keys.map((each: string, i: number) => (
              <TableRow
                key={each}
                sx={{
                  background:
                    mode === 'dark'
                      ? theme.palette.secondary.dark
                      : theme.palette.primary.light,
                }}
              >
                {icons && (
                  <TableCell>
                    {icons[i] ? <CheckCircleSharpIcon /> : <ErrorIcon />}
                  </TableCell>
                )}
                <TableCell sx={error ? { opacity: 0.4 } : null} data-testid={each.replace(/ /g,'')}>
                  {each}
                </TableCell>
                <TableCell sx={error ? { opacity: 0.4 } : null} align="right" data-testid={each.replace(/ /g,'-') + "-value"}>
                  {values[i]}
                </TableCell>
                {error && (
                  <TableCell align="right" sx={{ opacity: 0.4 }}>
                    {values[i]}
                  </TableCell>
                )}
                {!error && loading && (
                  <TableCell align="right">
                    <CircularProgress />
                  </TableCell>
                )}
                {error && !icons && (
                  <TableCell sx={{ opacity: 0.2 }} align="right">
                    <ErrorIcon />
                  </TableCell>
                )}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>
    </>
  );
};

TwoColSmallTable.defaultProps = {
  title: undefined,
  icons: [],
  marginBottom: false,
  error: undefined,
};
