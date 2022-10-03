import React, { useState, useEffect } from 'react';
import { Box, Card, CardContent, CardHeader, Grid, Typography } from '@mui/material';
import { ResultsTable } from 'src/components/RewardsPlayground/ResultsTable';
import { computeMixnodeRewardEstimation, getDelegationSummary } from 'src/requests';
import { NodeDetails } from 'src/components/RewardsPlayground/NodeDetail';
import { Inputs, CalculateArgs } from 'src/components/RewardsPlayground/Inputs';
import { TBondedMixnode, useBondingContext } from 'src/context';
import { isMixnode } from 'src/types';
import { SelectionChance } from '@nymproject/types';

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
    stakeSaturation: '',
    selectionProbability: 'Low' as SelectionChance,
  });

  const [defaultInputValues, setDefaultInputValues] = useState<DefaultInputValues>();

  const initialiseInputs = async (node: TBondedMixnode) => {
    const delegations = await getDelegationSummary();

    setDefaultInputValues({
      profitMargin: node.profitMargin,
      uptime: (node.uptime || 0).toString(),
      bond: node.bond.amount || '',
      delegations: delegations.total_delegations.amount,
      operatorCost: '',
    });
  };

  useEffect(() => {
    if (bondedNode && isMixnode(bondedNode)) {
      initialiseInputs(bondedNode);
    }
  }, []);

  const handleCalculate = async ({ bond, delegations, uptime }: CalculateArgs) => {
    try {
      const res = await computeMixnodeRewardEstimation({
        identity: bondedNode?.identityKey || '',
        uptime: parseInt(uptime, 10),
        isActive: true,
        pledgeAmount: Math.floor(+bond * 1_000_000),
        totalDelegation: Math.floor(+delegations * 1_000_000),
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
        stakeSaturation: '0',
        selectionProbability: 'High',
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
          {console.log(defaultInputValues?.uptime)}
          {defaultInputValues && <Inputs onCalculate={handleCalculate} defaultValues={defaultInputValues} />}
        </CardContent>
      </Card>
      <Grid container spacing={3}>
        <Grid item xs={12} md={8}>
          <ResultsTable results={results} />
        </Grid>
        <Grid item xs={12} md={4}>
          <NodeDetails saturation={results.stakeSaturation} selectionProbability={results.selectionProbability} />
        </Grid>
      </Grid>
    </Box>
  );
};
