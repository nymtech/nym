import { Box, CircularProgress, CircularProgressProps, Stack, Typography } from '@mui/material';
import { ResultsCard } from './ResultsCard';

const getPerformanceDescriptionAndColor = (score: number) => {
  const res: { description: string; color: CircularProgressProps['color'] } = { description: '', color: 'warning' };

  if (score >= 90) {
    res.description = 'Reliable node';
    res.color = 'success';
  }

  if (score >= 75 && score < 90) {
    res.description = 'Average node';
    res.color = 'warning';
  }

  if (score > 0 && score < 75) {
    res.description = 'Unreliable node';
    res.color = 'error';
  }

  return res;
};

export const NodeScore = ({ score }: { score: number }) => {
  const { color } = getPerformanceDescriptionAndColor(score);

  return (
    <ResultsCard label={<Typography fontWeight="bold">Node score</Typography>} detail="">
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
          value={100}
          size={250}
          sx={{ position: 'absolute', top: 0, left: 0, color: 'grey.200' }}
        />
        <CircularProgress
          variant="determinate"
          value={score}
          size={250}
          sx={{ position: 'absolute', top: 0, left: 0 }}
          color={color}
        />
        <Stack alignItems="center" gap={1}>
          <Typography fontWeight="bold" variant="h4">
            {Math.round(score)}%
          </Typography>
          <Typography>Performance Score</Typography>
        </Stack>
      </Box>
    </ResultsCard>
  );
};
