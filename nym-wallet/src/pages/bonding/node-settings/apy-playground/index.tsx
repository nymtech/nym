import React, { useState } from 'react';
import {
  Box,
  Button,
  Card,
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
import { computeMixnodeRewardEstimation } from 'src/requests';
import { SelectionChance } from '@nymproject/types';

const MAJOR_AMOUNT_FOR_CALCS = 1000;

const tableHeader = [
  { title: 'Estimated rewards', bold: true },
  { title: 'Per day' },
  { title: 'Per month' },
  { title: 'Per year' },
];

const colorMap: { [key in SelectionChance]: string } = {
  VeryLow: 'error.main',
  Low: 'error.main',
  Moderate: 'warning.main',
  High: 'success.main',
  VeryHigh: 'success.main',
};

const textMap: { [key in SelectionChance]: string } = {
  VeryLow: 'VeryLow',
  Low: 'Low',
  Moderate: 'Moderate',
  High: 'High',
  VeryHigh: 'Very high',
};

type Results = {
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

const InclusionProbability = ({ probability }: { probability: SelectionChance }) => (
  <Typography sx={{ color: colorMap[probability] }}>{textMap[probability]}</Typography>
);

const ResultsTable = ({ results }: { results: Results }) => {
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

const NodeDetails = ({
  saturation,
  selectionProbability,
}: {
  saturation: number;
  selectionProbability: SelectionChance;
}) => (
  <Card variant="outlined" sx={{ p: 1 }}>
    <CardContent>
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight="medium">Stake saturation</Typography>
        <Typography>{saturation}%</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight="medium">Selection probability</Typography>
        <InclusionProbability probability={selectionProbability} />
      </Stack>
    </CardContent>
  </Card>
);

export const ApyPlayground = () => {
  const [inputValues, setInputValues] = useState([
    { label: 'Profit margin', isPercentage: true, value: '0' },
    { label: 'Operator cost', value: '0' },
    { label: 'Bond', value: '0' },
    { label: 'Delegations', value: '0' },
    { label: 'Uptime', isPercentage: true, value: '0' },
  ]);
  const [results, setResults] = useState({
    total: { daily: '-', monthly: '-', yearly: '-' },
    operator: { daily: '-', monthly: '-', yearly: '-' },
    delegator: { daily: '-', monthly: '-', yearly: '-' },
  });

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setInputValues((current) => [
      ...current.map((input) => (input.label === e.target.name ? { ...input, value: e.target.value } : input)),
    ]);
  };

  const getInputValue = (inputName: string) => inputValues.find((input) => input.label === inputName);

  const handleCalculate = async () => {
    try {
      const res = await computeMixnodeRewardEstimation({
        identity: 'DLdMKLPywEy1vnu3yPrtXvzY7fw1puiiHpA9n9UQatiQ',
        uptime: +getInputValue('Uptime')!,
        isActive: true,
        pledgeAmount: Math.floor(+getInputValue('Bond')! * 1_000_000),
        totalDelegation: Math.floor(+getInputValue('Delegations')! * 1_000_000),
      });

      const operatorReward = (res.estimated_operator_reward / 1_000_000) * 24; // epoch_reward * 1 epoch_per_hour * 24 hours
      const delegatorsReward = (res.estimated_delegators_reward / 1_000_000) * 24;

      const operatorRewardScaled = MAJOR_AMOUNT_FOR_CALCS * (operatorReward / +getInputValue('Bond')!.value);
      const delegatorReward = MAJOR_AMOUNT_FOR_CALCS * (delegatorsReward / +getInputValue('Delegations')!.value!);

      setResults({
        total: {
          daily: (operatorRewardScaled + delegatorReward).toString(),
          monthly: ((operatorRewardScaled + delegatorReward) * 30).toString(),
          yearly: ((operatorRewardScaled + delegatorReward) * 365).toString(),
        },
        operator: {
          daily: operatorRewardScaled.toString(),
          monthly: (operatorRewardScaled * 30).toString(),
          yearly: (operatorRewardScaled * 365).toString(),
        },
        delegator: {
          daily: delegatorReward.toString(),
          monthly: (delegatorReward * 30).toString(),
          yearly: (delegatorReward * 365).toString(),
        },
      });
    } catch (e) {
      console.log(e);
    }
  };

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
          <ResultsTable results={results} />
        </Grid>
        <Grid item xs={12} md={4}>
          <NodeDetails saturation={10} selectionProbability="High" />
        </Grid>
      </Grid>
    </Box>
  );
};
