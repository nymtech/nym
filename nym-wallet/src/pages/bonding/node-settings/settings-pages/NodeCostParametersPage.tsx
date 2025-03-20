import React, { useState } from 'react';
import { Box, Button, Typography, Grid, TextField, Stack, InputAdornment } from '@mui/material';
import { TBondedNode } from 'src/context/bonding';
import { Error } from 'src/components/Error';
import { UpdateCostParametersModal } from 'src/components/Bonding/modals/UpdateCostParametersModal';
import { isMixnode, isNymNode } from 'src/types';

interface Props {
  bondedNode: TBondedNode;
  onConfirm: () => Promise<void>;
  onError: (e: string) => void;
}

export const NodeCostParametersPage = ({ bondedNode, onConfirm, onError }: Props) => {
  const [intervalOperatingCost, setIntervalOperatingCost] = useState('');
  const [profitMarginPercent, setProfitMarginPercent] = useState('');
  const [isConfirmed, setIsConfirmed] = useState(false);
  
  const handleIntervalOperatingCostChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    // Only allow numbers and decimals
    if (value === '' || /^[0-9]*\.?[0-9]*$/.test(value)) {
      setIntervalOperatingCost(value);
    }
  };

  const handleProfitMarginPercentChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    // Only allow numbers and decimals
    if (value === '' || /^[0-9]*\.?[0-9]*$/.test(value)) {
      setProfitMarginPercent(value);
    }
  };

  // Determine if the form is valid for submission
  const isFormValid = 
    intervalOperatingCost !== '' && 
    !isNaN(Number(intervalOperatingCost)) &&
    profitMarginPercent !== '' && 
    !isNaN(Number(profitMarginPercent)) &&
    Number(profitMarginPercent) >= 0 && 
    Number(profitMarginPercent) <= 100;

  // Only display warning for mixnodes or nymnodes
  const shouldDisplayWarning = isMixnode(bondedNode) || isNymNode(bondedNode);

  return (
    <Box sx={{ p: 3, minHeight: '450px' }}>
      <Grid container justifyContent="space-between">
        <Grid item xs={12} lg={4} sx={{ mb: 3 }}>
          <Stack gap={1}>
            <Typography variant="body1" fontWeight={600}>
              Update Cost Parameters
            </Typography>

            {shouldDisplayWarning && (
              <Grid item>
                <Typography variant="body2" sx={{ color: (theme) => theme.palette.nym.text.muted }}>
                  Updating cost parameters affects your node's economics in the network. Please ensure you understand the implications.
                </Typography>
              </Grid>
            )}
          </Stack>
        </Grid>
        <Grid item xs={12} lg={6}>
          <Stack gap={3}>
            {shouldDisplayWarning && (
              <Error message="Changes to cost parameters will affect your node's attractiveness to delegators and its profitability. Set these values carefully." />
            )}

            <Typography variant="body2">
              Enter your desired operating cost and profit margin parameters
            </Typography>

            <TextField
              fullWidth
              label="Interval Operating Cost"
              value={intervalOperatingCost}
              onChange={handleIntervalOperatingCostChange}
              InputLabelProps={{ shrink: true }}
              InputProps={{
                endAdornment: <InputAdornment position="end">unym</InputAdornment>,
              }}
              helperText="Operating cost in the current denomination (unym)"
            />

            <TextField
              fullWidth
              label="Profit Margin Percentage"
              value={profitMarginPercent}
              onChange={handleProfitMarginPercentChange}
              InputLabelProps={{ shrink: true }}
              InputProps={{
                endAdornment: <InputAdornment position="end">%</InputAdornment>,
              }}
              helperText="Input your profit margin (e.g., 20 for 20%)"
            />

            <Button
              size="large"
              variant="contained"
              fullWidth
              disabled={!isFormValid}
              onClick={() => {
                setIsConfirmed(true);
              }}
            >
              Continue
            </Button>
          </Stack>
        </Grid>
      </Grid>
      {isConfirmed && (
        <UpdateCostParametersModal
          node={bondedNode}
          intervalOperatingCost={intervalOperatingCost}
          profitMarginPercent={profitMarginPercent}
          onConfirm={async () => {
            setIsConfirmed(false);
            onConfirm();
          }}
          onClose={() => setIsConfirmed(false)}
          onError={onError}
        />
      )}
    </Box>
  );
};