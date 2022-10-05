import React from 'react';
import { Stack, Typography, Box, useTheme, Grid, LinearProgress, LinearProgressProps } from '@mui/material';
import { TBondedMixnode } from 'src/context';
import { Cell, Pie, PieChart, Legend, ResponsiveContainer } from 'recharts';
import { SelectionChance } from '@nymproject/types';
import { NymCard } from '../NymCard';
import { InfoTooltip } from '../InfoToolTip';

const LinearProgressWithLabel = (props: LinearProgressProps & { value: number }) => {
  const { value } = props;
  return (
    <Box sx={{ display: 'flex', alignItems: 'center' }}>
      <Box sx={{ minWidth: 30 }}>
        <Typography>{`${Math.round(value)}%`}</Typography>
      </Box>
      <Box sx={{ width: 40 }}>
        <LinearProgress sx={{ borderRadius: 4 }} color="success" variant="determinate" {...props} />
      </Box>
    </Box>
  );
};

const StatRow = ({
  label,
  tooltipText,
  value,
  withProgress,
}: {
  label: string;
  tooltipText: string;
  value: string | number;
  withProgress?: boolean;
}) => (
  <Stack direction="row" gap={1} justifyContent="space-between" alignItems="center" width="100%">
    <Stack direction="row" alignItems="center" gap={1} sx={{ color: (t) => t.palette.nym.text.muted }}>
      <InfoTooltip title={tooltipText} />
      <Typography>{label}</Typography>
    </Stack>
    {withProgress ? <LinearProgressWithLabel value={value as number} /> : <Typography>{value}</Typography>}
  </Stack>
);

const StatDivider = () => <Box borderTop="1px solid" borderColor="rgba(141, 147, 153, 0.2)" my={1} />;

export const NodeStats = ({ mixnode }: { mixnode: TBondedMixnode }) => {
  const {
    stakeSaturation,
    profitMargin,
    estimatedRewards,
    activeSetProbability,
    standbySetProbability,
    operatorCost,
    routingScore,
  } = mixnode;
  const theme = useTheme();
  const data = [
    { key: 'routingScore', value: routingScore },
    { key: 'rest', value: 100 - routingScore },
  ];
  const colors = [theme.palette.success.main, '#E6E6E6'];

  const getSetProbabilityLabel = (chance?: SelectionChance) => {
    if (!chance) return 'Unknown';
    switch (chance) {
      case 'High':
        return 'High';
      case 'Good':
        return 'Good';
      case 'Low':
        return 'Low';
      default:
        return 'Unknown';
    }
  };

  const renderLegend = () => (
    <Stack
      alignItems="center"
      maxWidth={200}
      width={200}
      sx={{
        borderBottom: '1px solid #E8E9EB',
      }}
    >
      <Typography sx={{ color: (t) => t.palette.nym.text.muted }}>Routing score</Typography>
      <Typography fontWeight={600} fontSize={28}>
        {routingScore}%
      </Typography>
    </Stack>
  );

  return (
    <NymCard
      borderless
      title={
        <Typography variant="h5" fontWeight={600}>
          Node stats
        </Typography>
      }
    >
      <Grid container spacing={4} direction="row" justifyContent="space-between" alignItems="flex-end">
        <Grid item xs={12} sm={12} md={6} lg={3}>
          <ResponsiveContainer width="100%" height={100}>
            <PieChart width={200} height={100}>
              <Pie
                cy={90}
                data={data}
                startAngle={180}
                endAngle={0}
                innerRadius={58}
                outerRadius={78}
                dataKey="value"
                stroke="none"
              >
                {data.map(({ key }, index) => (
                  <Cell key={`cell-${key}`} fill={colors[index]} />
                ))}
              </Pie>
              <Legend
                verticalAlign="bottom"
                content={renderLegend}
                wrapperStyle={{
                  display: 'flex',
                  justifyContent: 'center',
                }}
              />
            </PieChart>
          </ResponsiveContainer>
        </Grid>
        <Grid item xs={12} sm={12} md={6} lg={4}>
          <StatRow label="Profit margin" tooltipText="TODO" value={`${profitMargin}%`} />
          <StatDivider />
          <StatRow label="Operator Cost" tooltipText="TODO" value={operatorCost ? `${operatorCost} USD` : '-'} />
          <StatDivider />
          <StatRow
            label="Estimated reward"
            tooltipText="TODO"
            value={`~${estimatedRewards.amount} ${estimatedRewards.denom.toUpperCase()}`}
          />
        </Grid>
        <Grid item xs={12} sm={12} md={12} lg={5}>
          <StatRow label="Node stake saturation" tooltipText="TODO" value={stakeSaturation} withProgress />
          <StatDivider />
          <StatRow
            label="Chance of being in the active set"
            tooltipText="TODO"
            value={getSetProbabilityLabel(activeSetProbability)}
          />
          <StatDivider />
          <StatRow
            label="Chance of being in the standby set"
            tooltipText="TODO"
            value={getSetProbabilityLabel(standbySetProbability)}
          />
        </Grid>
      </Grid>
    </NymCard>
  );
};
