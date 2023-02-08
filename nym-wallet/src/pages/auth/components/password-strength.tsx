/* eslint-disable no-nested-ternary */
import React, { useEffect, useState } from 'react';
import zxcvbn, { ZXCVBNScore } from 'zxcvbn';
import { LockOutlined } from '@mui/icons-material';
import { LinearProgress, Stack, Typography, Box } from '@mui/material';

const colorMap = {
  4: 'success' as 'success',
  3: 'success' as 'success',
  2: 'warning' as 'warning',
  1: 'error' as 'error',
  0: 'error' as 'error',
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

export const PasswordStrength = ({ password }: { password: string }) => {
  const result = zxcvbn(password);

  return (
    <Stack spacing={0.5}>
      <LinearProgress variant="determinate" color={colorMap[result.score]} value={getPasswordStrength(result.score)} />
      <Box display="flex" alignItems="center">
        <LockOutlined sx={{ fontSize: 15, color: getColor(result.score) }} />
        <Typography variant="caption" sx={{ ml: 0.5, color: getColor(result.score) }}>
          {getText(result.score)}
        </Typography>
      </Box>
    </Stack>
  );
};
