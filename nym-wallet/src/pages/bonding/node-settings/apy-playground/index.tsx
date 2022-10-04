import React, { useState, useEffect } from 'react';
import { Box, Card, CardContent, CardHeader, Grid, Typography } from '@mui/material';
import { ResultsTable } from 'src/components/RewardsPlayground/ResultsTable';
import { computeMixnodeRewardEstimation, getDelegationSummary, getMixnodeRewardEstimation } from 'src/requests';
import { NodeDetails } from 'src/components/RewardsPlayground/NodeDetail';
import { Inputs, CalculateArgs } from 'src/components/RewardsPlayground/Inputs';
import { TBondedMixnode, useBondingContext } from 'src/context';
import { isMixnode } from 'src/types';
import { handleCalculate } from './utils';

const MAJOR_AMOUNT_FOR_CALCS = 1000;

export type DefaultInputValues = {
  profitMargin: string;
  uptime: string;
  bond: string;
  delegations: string;
  operatorCost: string;
};

export const ApyPlayground = () => {
  const { bondedNode } = useBondingContext();

  const [results, setResults] = useState({
    total: { daily: '-', monthly: '-', yearly: '-' },
    operator: { daily: '-', monthly: '-', yearly: '-' },
    delegator: { daily: '-', monthly: '-', yearly: '-' },
  });

  const [defaultInputValues, setDefaultInputValues] = useState<DefaultInputValues>();
  const [stakeSaturation, setStakeSaturation] = useState<string>();

  const initialise = async (node: TBondedMixnode) => {
    const delegations = await getDelegationSummary();
    const res = await getMixnodeRewardEstimation(node.id);

    setResults(handleCalculate(res.estimation.operator, res.estimation.delegates, res.estimation.total_node_reward));
    setStakeSaturation(node.stakeSaturation);
    setDefaultInputValues({
      profitMargin: node.profitMargin,
      uptime: (node.uptime || 0).toString(),
      bond: node.bond.amount || '',
      delegations: delegations.total_delegations.amount,
      operatorCost: Math.floor(res.estimation.operating_cost / 1_000_000).toString(),
    });
  };

  useEffect(() => {
    if (bondedNode && isMixnode(bondedNode)) {
      initialise(bondedNode);
    }
  }, []);

  const handleCalculateEstimate = async ({ bond, delegations, uptime }: CalculateArgs) => {
    try {
      const estimatedRewards = await computeMixnodeRewardEstimation({
        identity: bondedNode!.identityKey,
        performance: (parseInt(uptime, 10) / 100).toString(),
        isActive: true,
        pledgeAmount: Math.floor(+bond * 1_000_000),
        totalDelegation: Math.floor(+delegations * 1_000_000),
      });

      const estimationResult = handleCalculate(
        estimatedRewards.estimation.delegates,
        estimatedRewards.estimation.operator,
        estimatedRewards.estimation.total_node_reward,
      );

      setStakeSaturation('0');

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
      <Card variant="outlined" sx={{ p: 1, mb: 3 }}>
        <CardHeader
          title={
            <Typography variant="body2" fontWeight="medium">
              Estimation calculator
            </Typography>
          }
        />
        <CardContent>
          {defaultInputValues && <Inputs onCalculate={handleCalculateEstimate} defaultValues={defaultInputValues} />}
        </CardContent>
      </Card>
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
