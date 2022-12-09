import React, { useState, useEffect, useContext } from 'react';
import { Box, Card, CardContent, CardHeader, Grid, Typography } from '@mui/material';
import { decimalToPercentage, percentToDecimal } from '@nymproject/types';
import { ResultsTable } from 'src/components/RewardsPlayground/ResultsTable';
import { getDelegationSummary } from 'src/requests';
import { NodeDetails } from 'src/components/RewardsPlayground/NodeDetail';
import { Inputs, CalculateArgs } from 'src/components/RewardsPlayground/Inputs';
import { AppContext, TBondedMixnode } from 'src/context';
import { computeEstimate, computeStakeSaturation, handleCalculatePeriodRewards } from './utils';
import { useSnackbar } from 'notistack';

export type DefaultInputValues = {
  profitMargin: string;
  uptime: string;
  bond: string;
  delegations: string;
  operatorCost: string;
};

export const ApyPlayground = ({ bondedNode }: { bondedNode: TBondedMixnode }) => {
  const { enqueueSnackbar } = useSnackbar();

  const [results, setResults] = useState({
    total: { daily: '-', monthly: '-', yearly: '-' },
    operator: { daily: '-', monthly: '-', yearly: '-' },
    delegator: { daily: '-', monthly: '-', yearly: '-' },
  });

  const [defaultInputValues, setDefaultInputValues] = useState<DefaultInputValues>();
  const [stakeSaturation, setStakeSaturation] = useState<string>();

  const initialise = async (node: TBondedMixnode) => {
    try {
      const delegations = await getDelegationSummary();

      const { estimation } = await computeEstimate({
        mixId: node.mixId,
        uptime: node.uptime.toString(),
        profitMargin: node.profitMargin,
        operatorCost: node.operatorCost.amount,
        totalDelegation: delegations.total_delegations.amount,
        pledgeAmount: node.bond.amount,
      });

      setResults(
        handleCalculatePeriodRewards({
          estimatedOperatorReward: estimation.operator,
          estimatedDelegatorsReward: estimation.delegates,
        }),
      );

      setStakeSaturation(node.stakeSaturation);

      setDefaultInputValues({
        profitMargin: node.profitMargin,
        uptime: (node.uptime || 0).toString(),
        bond: node.bond.amount || '',
        delegations: delegations.total_delegations.amount,
        operatorCost: node.operatorCost.amount,
      });
    } catch (e) {
      enqueueSnackbar(e as string, { variant: 'error' });
    }
  };

  useEffect(() => {
    if (bondedNode) {
      initialise(bondedNode);
    }
  }, []);

  const handleCalculateEstimate = async ({ bond, delegations, uptime, profitMargin, operatorCost }: CalculateArgs) => {
    try {
      const { estimation, reward_params } = await computeEstimate({
        mixId: bondedNode.mixId,
        uptime: uptime,
        profitMargin: profitMargin,
        operatorCost: operatorCost,
        totalDelegation: delegations,
        pledgeAmount: bond,
      });

      const estimationResult = handleCalculatePeriodRewards({
        estimatedOperatorReward: estimation.operator,
        estimatedDelegatorsReward: estimation.delegates,
      });

      const computedStakeSaturation = computeStakeSaturation(
        bond,
        delegations,
        reward_params.interval.stake_saturation_point,
      );

      setStakeSaturation(decimalToPercentage(computedStakeSaturation.toString()));
      setResults(estimationResult);
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
      {defaultInputValues && (
        <Card variant="outlined" sx={{ p: 1, mb: 3 }}>
          <CardHeader
            title={
              <Typography variant="body2" fontWeight="medium">
                Estimation calculator
              </Typography>
            }
          />
          <CardContent>
            <Inputs onCalculate={handleCalculateEstimate} defaultValues={defaultInputValues} />
          </CardContent>
        </Card>
      )}
      <Grid container spacing={3}>
        <Grid item xs={12} md={8}>
          <ResultsTable results={results} />
        </Grid>
        <Grid item xs={12} md={4}>
          <NodeDetails saturation={stakeSaturation} />
        </Grid>
      </Grid>
    </Box>
  );
};
