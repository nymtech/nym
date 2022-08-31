import { useEffect, useState } from 'react';
import { Box, Button, Divider, Typography, TextField, InputAdornment, Grid, Alert } from '@mui/material';
import { TBondedMixnode, TBondedGateway } from '../../../../context/bonding';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

export const ParametersSettings = ({ bondedNode }: { bondedNode: TBondedMixnode | TBondedGateway }) => {
  const { profitMarginPercent, bond } = bondedNode;

  const [buttonActive, setButtonActive] = useState<boolean>(false);
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const [profitMargin, setProfitMargin] = useState<number | ''>(profitMarginPercent);
  const [operatorCost, setOperatorCost] = useState<string>(bond.amount);

  useEffect(() => {
    if (!profitMargin || !operatorCost || 0 >= profitMargin || 100 < profitMargin) {
      setButtonActive(false);
    }
  }, [profitMargin, operatorCost]);

  return (
    <Box sx={{ width: '79.88%', minHeight: '' }}>
      {buttonActive && (
        <Alert
          severity="info"
          sx={{
            px: 2,
            borderRadius: 0,
            bgcolor: 'background.default',
            color: 'info.dark',
            '& .MuiAlert-icon': { color: 'info.dark' },
          }}
        >
          <strong>Profit margin can be changed once a month, your changes will be applied in the next interval</strong>
        </Alert>
      )}
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
                label="Profit margin"
                value={profitMargin}
                onChange={(e) => {
                  setProfitMargin(parseInt(e.target.value) || '');
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
                label="Operator cost"
                value={operatorCost}
                onChange={(e) => {
                  setOperatorCost(e.target.value);
                  setButtonActive(true);
                }}
                fullWidth
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <span>{bond.denom.toUpperCase()}</span>
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
