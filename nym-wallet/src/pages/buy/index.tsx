import React from 'react';
import { Stack } from '@mui/material';
import { Tutorial } from 'src/components/Buy/Tutorial';
import { PageLayout } from '../../layouts';

export const BuyPage = () => (
  <PageLayout>
    <Stack spacing={3}>
      <Tutorial />
    </Stack>
  </PageLayout>
);
