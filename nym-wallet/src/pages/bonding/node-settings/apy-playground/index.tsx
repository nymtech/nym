import React, { useState } from 'react';
import {
  Box,
  Button,
  Card,
  CardActions,
  CardContent,
  CardHeader,
  Divider,
  Grid,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material';

const tableHeader = [
  { title: 'Estimated rewards', bold: true },
  { title: 'Per day' },
  { title: 'Per month' },
  { title: 'Per year' },
];

const tableRows = [
  { title: 'Total node reward', perDay: '10 NYM', perMonth: '300 NYM', perYear: '3600 NYM' },
  { title: 'Operator rewards', perDay: '10 NYM', perMonth: '300 NYM', perYear: '3600 NYM' },
  { title: 'Delegator rewards', perDay: '10 NYM', perMonth: '300 NYM', perYear: '3600 NYM' },
];

const ResultsTable = () => (
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
                <TableCell>{row.perDay}</TableCell>
                <TableCell>{row.perMonth}</TableCell>
                <TableCell>{row.perYear}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>
    </CardContent>
  </Card>
);

const NodeDetails = ({ saturation, selectionProbability }: { saturation: number; selectionProbability: string }) => (
  <Card variant="outlined" sx={{ p: 1 }}>
    <CardContent>
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight="medium">Stake saturation</Typography>
        <Typography>{saturation}%</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight="medium">Selection probability</Typography>
        <Typography>{selectionProbability}</Typography>
      </Stack>
    </CardContent>
  </Card>
);

export const ApyPlayground = () => {
  const [inputValues, setInputValues] = useState([
    { label: 'Profit margin', isPercentage: true, value: '' },
    { label: 'Operator cost', value: '' },
    { label: 'Bond', value: '' },
    { label: 'Delegations', value: '' },
    { label: 'Uptime', isPercentage: true, value: '' },
  ]);

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setInputValues((current) => [
      ...current.map((input) => (input.label === e.target.name ? { ...input, value: e.target.value } : input)),
    ]);
  };

  const handleCalculate = () => console.log(inputValues);

  return (
    <Box sx={{ p: 3 }}>
      <Typography fontWeight="medium" sx={{ mb: 1 }}>
        Playground
      </Typography>
      <Typography variant="body2" sx={{ color: 'grey.600', mb: 2 }}>
        This is your parameters playground - change the parameters below to see the node specific estimations in the
        table
      </Typography>
      <Card variant="outlined" sx={{ p: 1, mb: 3 }}>
        <CardHeader
          title={
            <Typography variant="body2" fontWeight="medium">
              Estimation calculator
            </Typography>
          }
        />
        <CardContent>
          <Grid container spacing={3} alignItems="center">
            {inputValues.map((input) => (
              <Grid item xs={12} lg={2}>
                <TextField
                  fullWidth
                  label={input.label}
                  name={input.label}
                  value={input.value}
                  onChange={handleInputChange}
                  InputProps={{
                    endAdornment: (
                      <Typography sx={{ color: 'grey.600' }}>{input.isPercentage ? '%' : 'NYM'}</Typography>
                    ),
                  }}
                />
              </Grid>
            ))}
            <Grid item xs={12} lg={2}>
              <Button variant="contained" disableElevation onClick={handleCalculate} size="large" fullWidth>
                Calculate
              </Button>
            </Grid>
          </Grid>
        </CardContent>
      </Card>
      <Grid container spacing={3}>
        <Grid item xs={12} md={8}>
          <ResultsTable />
        </Grid>
        <Grid item xs={12} md={4}>
          <NodeDetails saturation={10} selectionProbability="Low" />
        </Grid>
      </Grid>
    </Box>
  );
};
