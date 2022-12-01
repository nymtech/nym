import React, { useState } from 'react';
import { Button, Stack, Typography, Grid, useMediaQuery, useTheme } from '@mui/material';
import { Tune as TuneIcon, BorderColor as BorderColorIcon, Paid as PaidIcon } from '@mui/icons-material';
import { NymCard } from '../NymCard';
import { SignMessageModal } from './SignMessageModal';

// TODO retrieve this value from env
const EXCHANGE_URL = 'https://buy.nymtech.net';

const borderColor = 'rgba(141, 147, 153, 0.2)';

const TutorialStep = ({
  step,
  title,
  text,
  icon,
  borderRight,
  borderBottom,
  fixTitleHeight,
}: {
  step: number;
  title: string;
  text: React.ReactNode;
  icon: React.ReactNode;
  borderRight?: boolean;
  borderBottom?: boolean;
  fixTitleHeight?: boolean;
}) => (
  <Grid
    item
    md={4}
    pb={2}
    pr={1}
    sx={{
      borderRight: borderRight ? `1px solid ${borderColor}` : null,
      borderBottom: borderBottom ? `1px solid ${borderColor}` : null,
    }}
  >
    <Stack gap={2}>
      <Stack direction="row" gap={1} alignItems="center">
        {icon}
        <Typography fontWeight={600} fontSize="12px">
          {`STEP ${step}`}
        </Typography>
      </Stack>
      <Typography
        fontWeight={600}
        variant="h6"
        sx={{
          minHeight: fixTitleHeight ? '40px' : undefined,
        }}
      >
        {title}
      </Typography>
      {text}
    </Stack>
  </Grid>
);

export const Tutorial = () => {
  const [showSignModal, setShowSignModal] = useState(false);
  const theme = useTheme();
  const showBorder = useMediaQuery(theme.breakpoints.up('md'));
  const fixTitleHeight = useMediaQuery(theme.breakpoints.down('lg'));

  return (
    <NymCard borderless title="Buy NYM with BTC without KYC" sx={{ mt: 4 }}>
      <Typography mb={2}>
        Follow below 3 steps to quickly and easily buy NYM tokens. You can purchase up to 1000 Swiss Francs per day
        without KYC.
      </Typography>
      {showSignModal && <SignMessageModal onClose={() => setShowSignModal(false)} />}
      <Grid
        container
        spacing={2}
        mb={3}
        mt={3}
        sx={{
          border: `1px solid ${borderColor}`,
          borderRadius: '8px',
        }}
      >
        <TutorialStep
          step={1}
          title="Define purchase details"
          icon={<TuneIcon fontSize="small" />}
          text={
            <Typography sx={{ color: (t) => t.palette.nym.text.muted }}>
              Click on{' '}
              <Typography display="inline" fontWeight={600}>
                Buy NYM
              </Typography>{' '}
              button and follow the steps in the browser window that opens. You will be asked for purchase details i.e.
              amount, wallet address, etc.
            </Typography>
          }
          borderRight={showBorder}
          borderBottom={!showBorder}
          fixTitleHeight={fixTitleHeight}
        />
        <TutorialStep
          step={2}
          title="Sign a message with your Nym wallet"
          icon={<BorderColorIcon fontSize="small" />}
          text={
            <Typography sx={{ color: (t) => t.palette.nym.text.muted }}>
              When asked for signature, copy the message and sign it using{' '}
              <Typography display="inline" fontWeight={600}>
                Sign message
              </Typography>{' '}
              button below. Then copy and paste your signature back in the browser window.
            </Typography>
          }
          borderRight={showBorder}
          borderBottom={!showBorder}
          fixTitleHeight={fixTitleHeight}
        />
        <TutorialStep
          step={3}
          title="Make BTC tx and receive NYM"
          icon={<PaidIcon fontSize="small" />}
          text={
            <Typography sx={{ color: (t) => t.palette.nym.text.muted }}>
              Send BTC to the given address. When the transaction is confirmed your purchased NYM tokens will be
              transferred in your wallet.
            </Typography>
          }
          fixTitleHeight={fixTitleHeight}
        />
      </Grid>
      <Stack direction="row" gap={2} justifyContent="flex-end" mt={5}>
        <Button variant="outlined" size="large" onClick={() => setShowSignModal(true)}>
          Sign message
        </Button>
        <Button variant="contained" size="large" href={EXCHANGE_URL} target="_blank">
          Buy NYM
        </Button>
      </Stack>
    </NymCard>
  );
};
