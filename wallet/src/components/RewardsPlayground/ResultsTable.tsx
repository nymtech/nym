import React from 'react';
import {
  Card,
  CardContent,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material';

export type Results = {
  operator: {
    daily: string;
    monthly: string;
    yearly: string;
  };
  delegator: {
    daily: string;
    monthly: string;
    yearly: string;
  };
  total: {
    daily: string;
    monthly: string;
    yearly: string;
  };
};

const tableHeader = [
  { title: 'Estimated rewards', bold: true },
  { title: 'Per day' },
  { title: 'Per month' },
  { title: 'Per year' },
];

export const ResultsTable = ({ results }: { results: Results }) => {
  const tableRows = [
    { title: 'Total node reward', ...results.total },
    { title: 'Operator rewards', ...results.operator },
    { title: 'Delegator rewards', ...results.delegator },
  ];

  return (
    <Card variant="outlined" sx={{ p: 1 }}>
      <CardContent>
        <TableContainer>
          <Table>
            <TableHead>
              <TableRow>
                {tableHeader.map((header) => (
                  <TableCell>
                    <Typography fontWeight={header.bold ? 'bold' : 'regular'}>{header.title}</Typography>
                  </TableCell>
                ))}
              </TableRow>
            </TableHead>
            <TableBody>
              {tableRows.map((row) => (
                <TableRow>
                  <TableCell>{row.title}</TableCell>
                  <TableCell>{row.daily}</TableCell>
                  <TableCell>{row.monthly}</TableCell>
                  <TableCell>{row.yearly}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </CardContent>
    </Card>
  );
};
