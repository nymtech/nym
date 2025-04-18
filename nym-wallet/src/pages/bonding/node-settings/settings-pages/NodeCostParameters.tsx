import React, { useState, useEffect } from 'react';
import { Box, Button, Typography, Grid, TextField, Stack, InputAdornment } from '@mui/material';
import { FeeDetails } from '@nymproject/types';
import { TBondedNode } from 'src/context/bonding';
import { Error } from 'src/components/Error';
import { UpdateCostParametersModal } from 'src/components/Bonding/modals/NodeCostParametersModals';
import { isMixnode, isNymNode } from 'src/types';
import { useBondingContext } from 'src/context';

interface Props {
  bondedNode: TBondedNode;
  onConfirm: () => Promise<void>;
  onError: (e: string) => void;
  onUpdateData?: (profitMarginPercent: string, intervalOperatingCost: string, fee?: FeeDetails) => void;
}

export const NodeCostParametersPage = ({ bondedNode, onConfirm, onError, onUpdateData }: Props) => {
  const { updateCostParameters } = useBondingContext();
  const [intervalOperatingCost, setIntervalOperatingCost] = useState('');
  const [profitMarginPercent, setProfitMarginPercent] = useState('');
  const [isFormValid, setIsFormValid] = useState(false);
  const [isConfirmed, setIsConfirmed] = useState(false);
  const [fee, setFee] = useState<FeeDetails | undefined>(undefined);

  // Load initial values from the bonded node if available
  useEffect(() => {
    if (bondedNode) {
      if (isMixnode(bondedNode) && bondedNode.operatorCost) {
        setIntervalOperatingCost(bondedNode.operatorCost.amount);
      }
      if (isMixnode(bondedNode) && bondedNode.profitMargin) {
        setProfitMarginPercent(bondedNode.profitMargin);
      }
    }
  }, [bondedNode]);

  useEffect(() => {
    if (onUpdateData && isFormValid) {
      onUpdateData(profitMarginPercent, intervalOperatingCost, fee);
    }
  }, [profitMarginPercent, intervalOperatingCost, fee, isFormValid, onUpdateData]);

  const handleIntervalOperatingCostChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { value } = e.target;
    if (value === '' || /^[0-9]*\.?[0-9]*$/.test(value)) {
      setIntervalOperatingCost(value);
    }
  };

  const handleProfitMarginPercentChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { value } = e.target;
    if (value === '' || /^[0-9]*\.?[0-9]*$/.test(value)) {
      setProfitMarginPercent(value);
    }
  };

  useEffect(() => {
    const isOperatingCostValid = intervalOperatingCost !== '' && !Number.isNaN(Number(intervalOperatingCost));
    const isProfitMarginValid =
      profitMarginPercent !== '' &&
      !Number.isNaN(Number(profitMarginPercent)) &&
      Number(profitMarginPercent) >= 20 &&
      Number(profitMarginPercent) <= 50;

    setIsFormValid(isOperatingCostValid && isProfitMarginValid);
  }, [intervalOperatingCost, profitMarginPercent]);

  const shouldDisplayWarning = isMixnode(bondedNode) || isNymNode(bondedNode);

  const handleModalConfirm = async () => {
    try {
      const uNymAmount = String(Math.floor(Number(intervalOperatingCost) * 1000000));

      if (onUpdateData) {
        onUpdateData(profitMarginPercent, intervalOperatingCost, fee);
      }

      await updateCostParameters(profitMarginPercent, uNymAmount, fee);
      setIsConfirmed(false);
      onConfirm();
    } catch (error) {
      onError(error as string);
    }
  };

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
                  Updating cost parameters affects your node&apos;s economics in the network. Please ensure you
                  understand the implications.
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

            <Typography variant="body2">Enter your desired operating cost and profit margin parameters</Typography>

            <TextField
              fullWidth
              label="Interval Operating Cost"
              value={intervalOperatingCost}
              onChange={handleIntervalOperatingCostChange}
              InputLabelProps={{ shrink: true }}
              InputProps={{
                endAdornment: <InputAdornment position="end">nym</InputAdornment>,
              }}
              helperText="Operating cost in the current denomination (nym)"
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
              helperText="Input your profit margin (must be between 20% and 50%)"
              error={
                profitMarginPercent !== '' && (Number(profitMarginPercent) < 20 || Number(profitMarginPercent) > 50)
              }
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
          onConfirm={handleModalConfirm}
          onClose={() => setIsConfirmed(false)}
          onError={onError}
          onFeeUpdate={setFee}
        />
      )}
    </Box>
  );
};
