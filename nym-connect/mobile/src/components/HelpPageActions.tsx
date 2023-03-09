import { ArrowBack, ArrowForward } from '@mui/icons-material';
import { Box, Button } from '@mui/material';
import React from 'react';

export const HelpPageActions = ({ onNext, onPrev }: { onNext?: () => void; onPrev?: () => void }) => (
  <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
    {onPrev ? (
      <Button onClick={onPrev} color="inherit" startIcon={<ArrowBack color="inherit" style={{ fontSize: 22 }} />}>
        Back
      </Button>
    ) : (
      <div />
    )}
    {onNext && (
      <Button onClick={onNext} endIcon={<ArrowForward style={{ fontSize: 22 }} />}>
        Next
      </Button>
    )}
  </Box>
);
