import zxcvbn, { ZXCVBNScore } from 'zxcvbn';
import { LockOutlined } from '@mui/icons-material';
import { LinearProgress, Stack, Typography, Box } from '@mui/material';

const colorMap = {
  4: 'success' as const,
  3: 'success' as const,
  2: 'warning' as const,
  1: 'error' as const,
  0: 'error' as const,
};

const getText = (score: ZXCVBNScore) => {
  switch (score) {
    case 4:
      return 'Very strong password';
    case 3:
      return 'Strong password';
    case 2:
      return 'Average password';
    case 1:
      return 'Weak password';
    case 0:
      return 'Very weak password';
    default:
      return '';
  }
};

const getColor = (score: ZXCVBNScore) => {
  switch (score) {
    case 4:
      return 'success.main';
    case 3:
      return 'success.main';
    case 2:
      return 'warning.main';
    case 1:
      return 'error.main';
    case 0:
      return 'error.main';
    default:
      return 'grey.500';
  }
};

const getPasswordStrength = (score: ZXCVBNScore) => {
  switch (score) {
    case 4:
      return 100;
    case 3:
      return 75;
    case 2:
      return 50;
    case 1:
      return 25;
    default:
      return 0;
  }
};

export const PasswordStrength = ({
  password = '',
  withWarnings,
  handleIsSafePassword,
}: {
  password: string;
  withWarnings?: boolean;
  handleIsSafePassword: (isSafe: boolean) => void;
}) => {
  const result = zxcvbn(password);

  handleIsSafePassword(result.score > 1);

  if (!password.length) return null;

  return (
    <Stack spacing={0.5}>
      <LinearProgress variant="determinate" color={colorMap[result.score]} value={getPasswordStrength(result.score)} />
      <Box display="flex" alignItems="center" justifyContent="space-between">
        <Box display="flex" alignItems="center">
          <LockOutlined sx={{ fontSize: 15, color: getColor(result.score) }} />
          <Typography variant="caption" sx={{ ml: 0.5, color: getColor(result.score) }}>
            {getText(result.score)}
          </Typography>
        </Box>
        {withWarnings && result.feedback.warning && (
          <Typography variant="caption">{result.feedback.warning}</Typography>
        )}
      </Box>
    </Stack>
  );
};
