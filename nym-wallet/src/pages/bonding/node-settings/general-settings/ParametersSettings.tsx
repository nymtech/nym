import { useEffect, useState } from 'react';
import { Box, Button, Divider, Typography, TextField, InputAdornment, Grid } from '@mui/material';

import { SimpleModal } from '../../../../components/Modals/SimpleModal';

type TSettingItem = {
  id: string;
  title: string;
  value: number | string;
};

const currentProfitMargin: TSettingItem = { id: 'profit-margin', title: 'Profit margin', value: 10 };
const currentOperatorCost: TSettingItem = { id: 'operator-cost', title: 'Operator cost', value: 40 };

export const ParametersSettings = () => {
  const [buttonActive, setButtonActive] = useState<boolean>(false);
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const [profitMargin, setProfitMargin] = useState<TSettingItem>(currentProfitMargin);
  const [operatorCost, setOperatorCost] = useState<TSettingItem>(currentOperatorCost);

  useEffect(() => {
    if (!profitMargin.value || !operatorCost.value) {
      setButtonActive(false);
    }
  }, [profitMargin, operatorCost]);

  return (
    <Box sx={{ width: 0.78, minHeight: '' }}>
      <Grid container direction="column">
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item direction="column">
            <Typography sx={{ fontSize: 16, fontWeight: 600, mb: 1 }}>Profit Margin</Typography>
            <Typography
              sx={{
                fontSize: 14,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Profit margin can be changed once a month
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" maxWidth="348px">
            <Grid item width={1} spacing={3}>
              <TextField
                type="input"
                label={profitMargin.title}
                value={profitMargin.value}
                onChange={(e) => {
                  console.log('parseInt(e.target.value)', { ...profitMargin, value: parseInt(e.target.value) });
                  setProfitMargin({ ...profitMargin, value: parseInt(e.target.value) || '' });
                  setButtonActive(true);
                }}
                fullWidth
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <span>NYM</span>
                    </InputAdornment>
                  ),
                }}
              />
            </Grid>
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item direction="column">
            <Typography sx={{ fontSize: 16, fontWeight: 600, mb: 1 }}>Operator cost</Typography>
            <Typography
              sx={{
                fontSize: 14,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Lock Wallet after a certain time
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" maxWidth="348px">
            <Grid item width={1} spacing={3}>
              <TextField
                type="input"
                label={operatorCost.title}
                value={operatorCost.value}
                onChange={(e) => {
                  console.log('nym', { ...operatorCost, value: parseInt(e.target.value) });
                  setOperatorCost({ ...operatorCost, value: parseInt(e.target.value) || '' });
                  setButtonActive(true);
                }}
                fullWidth
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <span>%</span>
                    </InputAdornment>
                  ),
                }}
              />
            </Grid>
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid container justifyContent="end">
          <Button
            size="large"
            variant="contained"
            disabled={!buttonActive}
            onClick={() => setOpenConfirmationModal(true)}
            sx={{ m: 3, width: '320px' }}
          >
            Save all changes
          </Button>
        </Grid>
      </Grid>
      <SimpleModal
        open={openConfirmationModal}
        header="Your changes will take place 
        in the next interval"
        okLabel="close"
        hideCloseIcon
        displayInfoIcon
        onOk={async () => {
          await setOpenConfirmationModal(false);
        }}
        buttonFullWidth
        sx={{
          width: '320px',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
        }}
        headerStyles={{
          width: '100%',
          mb: 1,
          textAlign: 'center',
          color: 'info.dark',
          fontSize: 16,
          textTransform: 'capitalize',
        }}
        subHeaderStyles={{
          m: 0,
        }}
      />
    </Box>
  );
};
