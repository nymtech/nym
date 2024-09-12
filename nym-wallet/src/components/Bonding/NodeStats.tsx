import React from 'react';
import { Stack, Typography, Box, useTheme, Grid, LinearProgress, LinearProgressProps, Button } from '@mui/material';
import { useNavigate } from 'react-router-dom';
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
  textColor,
  progressValue,
}: {
  label: string;
  tooltipText?: string;
  value: string | number;
  textColor?: string;
  progressValue?: number;
}) => (
  <Stack direction="row" gap={1} justifyContent="space-between" alignItems="center" width="100%">
    <Stack direction="row" alignItems="center" gap={1} sx={{ color: (t) => t.palette.nym.text.muted }}>
      {tooltipText && <InfoTooltip title={tooltipText} />}
      <Typography>{label}</Typography>
    </Stack>
    {typeof progressValue === 'number' ? (
      <LinearProgressWithLabel value={progressValue} />
    ) : (
      <Typography color={textColor}>{value}</Typography>
    )}
  </Stack>
);

export const NodeStats = ({ mixnode }: { mixnode: TBondedMixnode }) => {
  const { activeSetProbability, routingScore } = mixnode;
  const theme = useTheme();
  const navigate = useNavigate();

  // clamp routing score to [0-100]
  const score = Math.min(Math.max(routingScore || 0, 0), 100);

  const data = [
    { key: 'routingScore', value: score },
    { key: 'rest', value: 100 - score },
  ];
  const colors = [theme.palette.success.main, theme.palette.nym.nymWallet.chart.grey];

  const getSetProbabilityLabel = (chance?: SelectionChance): { value: string; color?: string } => {
    switch (chance) {
      case 'High':
        return { value: 'High', color: theme.palette.nym.success };
      case 'Good':
        return { value: 'Good' };
      case 'Low':
        return { value: 'Low', color: theme.palette.nym.red };
      default:
        return { value: 'Unknown' };
    }
  };

  const handleGoToTestNode = () => {
    navigate('/bonding/node-settings', { state: 'test-node' });
  };

  const renderLegend = () => (
    <Stack
      alignItems="center"
      maxWidth={200}
      width={200}
      sx={{
        borderBottom: `1px solid ${theme.palette.nym.nymWallet.chart.grey}`,
      }}
    >
      <Typography fontWeight={600} fontSize={24} mb={1}>
        {routingScore}%
      </Typography>
    </Stack>
  );

  const activeSetProb = getSetProbabilityLabel(activeSetProbability);

  return (
    <Grid container spacing={4} direction="row" justifyContent="space-between" alignItems="flex-end">
      <Grid item xs={12} sm={8} md={7} lg={6}>
        <NymCard
          borderless
          title={
            <Typography variant="h5" fontWeight={600}>
              Node stats
            </Typography>
          }
          Action={
            <Button size="small" variant="contained" disableElevation onClick={handleGoToTestNode}>
              Test node
            </Button>
          }
        >
          <Stack justifyContent="center" alignItems="center" mb={2}>
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
            <Typography color="nym.text.muted">Routing score</Typography>
          </Stack>
          <StatRow
            label="Chance of being in the active set"
            value={activeSetProb.value}
            textColor={activeSetProb.color}
          />
        </NymCard>
      </Grid>
      <Grid item xs={12} sm={4} md={5} lg={7} />
    </Grid>
  );
};
