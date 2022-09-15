import React, { useState } from 'react';
import { Box, Button, Stack, Typography, Grid, Link } from '@mui/material';
import { NymCard } from '../NymCard';
import { ModalDivider } from '../Modals/ModalDivider';
import { SignMessageModal } from './SignMessageModal';

export const Tutorial = () => {
  const [showSignModal, setShowSignModal] = useState(false);

  return (
    <Box>
      {showSignModal && <SignMessageModal onClose={() => setShowSignModal(false)} />}
      <Stack direction="row" justifyContent="space-between" sx={{ mb: 3, mt: 2 }}>
        <Box>
          <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>
            How to buy NYM with Bity?
          </Typography>
          <Typography variant="subtitle1">Follow these 3 steps below to quickly and easily buy NYM tokens</Typography>
        </Box>
        <Button variant="contained" color="primary" onClick={() => {}}>
          Buy Nym
        </Button>
      </Stack>
      <Box sx={{ flexGrow: 1 }}>
        <Grid container spacing={2}>
          <Grid item md={4}>
            <NymCard title="1. Define the purchase amount" sx={{ height: 440 }} sxTitle={{ fontSize: 16 }} borderless>
              <ModalDivider />
              <Typography>
                Click on{' '}
                <Typography display="inline" fontWeight={600}>
                  Buy NYM button to go to Bity’s website.
                </Typography>{' '}
                Select the amount and currency for your purchase. Follow the steps and provide the required info i.e.{' '}
                <Typography display="inline" fontWeight={600}>
                  IBAN, wallet address, etc.
                </Typography>
              </Typography>
            </NymCard>
          </Grid>
          <Grid item md={4}>
            <NymCard
              title="2. Sign a message with your Nym wallet"
              sx={{ height: 440 }}
              sxTitle={{ fontSize: 16 }}
              borderless
            >
              <ModalDivider />
              <Typography>
                When asked for signature, copy Bity’s message and{' '}
                {/* eslint-disable-next-line jsx-a11y/anchor-is-valid */}
                <Link
                  component="button"
                  variant="body1"
                  sx={{ fontWeight: 600 }}
                  onClick={() => setShowSignModal(true)}
                >
                  sign it using this link
                </Link>
                {'. '}
                Then{' '}
                <Typography display="inline" fontWeight={600}>
                  copy and paste your signature on Bity website
                </Typography>{' '}
                as shown above.
              </Typography>
            </NymCard>
          </Grid>
          <Grid item md={4}>
            <NymCard
              title="3. Transfer funds and receive NYM"
              sx={{ height: 440 }}
              sxTitle={{ fontSize: 16 }}
              borderless
            >
              <ModalDivider />
              <Typography>
                Make the transfer to Bity’s address. Once Bity receives the amount and transaction is confirmed they
                will{' '}
                <Typography display="inline" fontWeight={600}>
                  deposit NYM tokens to your wallet.
                </Typography>
              </Typography>
            </NymCard>
          </Grid>
        </Grid>
      </Box>
    </Box>
  );
};
