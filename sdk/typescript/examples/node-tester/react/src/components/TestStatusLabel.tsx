import React from 'react';
import { Chip } from '@mui/material';
import { HourglassTop, ErrorOutline, CheckCircleOutline, WarningAmber } from '@mui/icons-material';
import { TestState } from 'src/hooks/useNodeTesterClient';

const getColor = (state: TestState) => {
  switch (state) {
    case 'Connecting':
      return 'warning';
    case 'Error':
      return 'error';
    case 'Ready':
      return 'success';
    default:
      return 'warning';
  }
};

const getIcon = (state: TestState) => {
  switch (state) {
    case 'Connecting':
      return <HourglassTop />;
    case 'Error':
      return <ErrorOutline />;
    case 'Ready':
      return <CheckCircleOutline />;
    default:
      return <WarningAmber />;
  }
};

export const TestStatusLabel = ({ state }: { state: TestState }) => (
  <Chip label={state} color={getColor(state)} icon={getIcon(state)} sx={{ color: 'white' }} />
);
