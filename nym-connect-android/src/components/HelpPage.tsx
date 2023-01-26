import React from 'react';
import { Stack, Typography } from '@mui/material';
import { HelpPageActions } from './HelpPageActions';
import { HelpImage } from './HelpPageImage';
import { StepIndicator } from './HelpPageStepIndicator';

export const HelpPage = ({
  step,
  description,
  img,
  onNext,
  onPrev,
}: {
  step: number;
  description: string;
  img: any;
  onNext?: () => void;
  onPrev?: () => void;
}) => (
  <Stack justifyContent="space-between" sx={{ height: '100%' }}>
    <Stack gap={3}>
      <StepIndicator step={step} />
      <Typography variant="body2" color="white" fontWeight="bold">
        How to connect guide {step}/4
      </Typography>
      <Typography variant="body2" sx={{ color: 'grey.400' }}>
        {description}
      </Typography>
      <HelpImage img={img} imageDescription="select a provider" />
    </Stack>
    <HelpPageActions onNext={onNext} onPrev={onPrev} />
  </Stack>
);
