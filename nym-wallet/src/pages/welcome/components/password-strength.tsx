import React, { useEffect, useState } from 'react';
import { LockOutlined } from '@mui/icons-material';
import { LinearProgress, Stack, Typography, Box } from '@mui/material';

type TStrength = 'weak' | 'medium' | 'strong' | 'init';

const strong = /^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9])(?=.*[!@#$%^&*])(?=.{8,})/;
const medium = /^(((?=.*[a-z])(?=.*[A-Z]))|((?=.*[a-z])(?=.*[0-9]))|((?=.*[A-Z])(?=.*[0-9])))(?=.{6,})/;

const colorMap = {
  init: 'inherit' as 'inherit',
  weak: 'error' as 'error',
  medium: 'warning' as 'warning',
  strong: 'success' as 'success',
};

const getText = (strength: TStrength) => {
  switch (strength) {
    case 'strong':
      return 'Strong password';
    case 'medium':
      return 'Medium strength password';
    case 'weak':
      return 'Weak password';
    default:
      return 'Password strength';
  }
};

const getTextColor = (strength: TStrength) => {
  switch (strength) {
    case 'strong':
      return 'success.main';
    case 'medium':
      return 'warning.main';
    case 'weak':
      return 'error.main';
    default:
      return 'grey.500';
  }
};

const getPasswordStrength = (strength: TStrength) => {
  switch (strength) {
    case 'strong':
      return 100;
    case 'medium':
      return 50;
    default:
      return 0;
  }
};

export const PasswordStrength = ({
  password,
  onChange,
}: {
  password: string;
  onChange: (isStrong: boolean) => void;
}) => {
  const [strength, setStrength] = useState<TStrength>('init');

  useEffect(() => {
    if (password.length === 0) {
      setStrength('init');
      return;
    }

    if (password.match(strong)) {
      setStrength('strong');
      return;
    }

    if (password.match(medium)) {
      setStrength('medium');
      return;
    }
    setStrength('weak');
  }, [password]);

  useEffect(() => {
    if (strength === 'strong') {
      onChange(true);
    } else {
      onChange(false);
    }
  }, [strength]);

  return (
    <Stack spacing={0.5}>
      <LinearProgress variant="determinate" color={colorMap[strength]} value={getPasswordStrength(strength)} />
      <Box display="flex" alignItems="center">
        <LockOutlined sx={{ fontSize: 15, color: getTextColor(strength) }} />
        <Typography variant="caption" sx={{ ml: 0.5, color: getTextColor(strength) }}>
          {getText(strength)}
        </Typography>
      </Box>
    </Stack>
  );
};
