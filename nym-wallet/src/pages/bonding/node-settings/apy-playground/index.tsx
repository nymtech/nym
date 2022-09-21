import React, { useState } from 'react';
import { Box, Button, Card, CardContent, CardHeader, Grid, TextField, Typography } from '@mui/material';
import { ResultsTable } from 'src/components/RewardsPlayground/ResultsTable';
import { computeMixnodeRewardEstimation } from 'src/requests';
import { NodeDetails } from 'src/components/RewardsPlayground/NodeDetail';
import { Inputs, InputValues } from 'src/components/RewardsPlayground/Inputs';

const MAJOR_AMOUNT_FOR_CALCS = 1000;

export const ApyPlayground = () => {
  const [inputValues, setInputValues] = useState<InputValues>([
    { label: 'Profit margin', name: 'profitMargin', isPercentage: true },
    { label: 'Operator cost', name: 'operatorCost' },
    { label: 'Bond', name: 'bond' },
    { label: 'Delegations', name: 'delegations' },
    { label: 'Uptime', name: 'uptime', isPercentage: true },
  ]);

  const [results, setResults] = useState({
    total: { daily: '-', monthly: '-', yearly: '-' },
    operator: { daily: '-', monthly: '-', yearly: '-' },
    delegator: { daily: '-', monthly: '-', yearly: '-' },
  });

  const getInputValue = (inputName: string) => inputValues.find((input) => input.name === inputName);

  const handleCalculate = async () => {
    try {
      const res = await computeMixnodeRewardEstimation({
        identity: 'DLdMKLPywEy1vnu3yPrtXvzY7fw1puiiHpA9n9UQatiQ',
        uptime: 0,
        isActive: true,
        pledgeAmount: Math.floor(0 * 1_000_000),
        totalDelegation: Math.floor(0 * 1_000_000),
      });

      const operatorReward = (res.estimated_operator_reward / 1_000_000) * 24; // epoch_reward * 1 epoch_per_hour * 24 hours
      const delegatorsReward = (res.estimated_delegators_reward / 1_000_000) * 24;

      const operatorRewardScaled = MAJOR_AMOUNT_FOR_CALCS * (operatorReward / 0);
      const delegatorReward = MAJOR_AMOUNT_FOR_CALCS * (delegatorsReward / 0);

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
          <Inputs inputValues={inputValues} onCalculate={handleCalculate} />
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
