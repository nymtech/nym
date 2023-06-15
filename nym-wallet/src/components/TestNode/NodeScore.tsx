import React from 'react';
import { Box, CircularProgress, Stack, Typography } from '@mui/material';
import { ResultsCard } from './ResultsCard';

const getPerformanceDescription = (score: number) => {
  if (score >= 90) return 'Reliable node';

  if (score >= 75 && score < 90) return 'Average node';

  if (score > 0 && score < 75) return 'Unreliable node';

  return '';
};

export const NodeScore = ({ score }: { score: number }) => {
  const performance = getPerformanceDescription(score);

  return (
    <ResultsCard label="Node score" detail={performance} isOk={performance === 'Reliable node'}>
      <Box
        sx={{
          display: 'flex',
          position: 'relative',
          width: 250,
          height: 250,
          justifyContent: 'center',
          alignItems: 'center',
          mx: 'auto',
          mt: 4,
        }}
      >
        <CircularProgress
          variant="determinate"
          value={score}
          size={250}
          sx={{ position: 'absolute', top: 0, left: 0 }}
          color={performance === 'Unreliable node' ? 'error' : performance === 'Reliable node' ? 'success' : 'warning'}
        />
        <Stack alignItems="center" gap={1}>
          <Typography fontWeight="bold" variant="h4">
            {score}%
          </Typography>
          <Typography>Performance Score</Typography>
        </Stack>
      </Box>
    </ResultsCard>
  );
};
