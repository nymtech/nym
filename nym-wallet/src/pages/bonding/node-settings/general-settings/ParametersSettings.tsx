import { useState, useEffect } from 'react';
import { Button, Divider, Typography, TextField, InputAdornment, Grid, Alert } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { TBondedMixnode, TBondedGateway } from '../../../../context/bonding';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

export const ParametersSettings = ({ bondedNode }: { bondedNode: TBondedMixnode | TBondedGateway }) => {
  const { profitMargin, bond } = bondedNode;

  const [buttonActive, setButtonActive] = useState<boolean>(false);
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const [profitMarginPercent, setProfitMarginPercent] = useState<string>(profitMargin);
  const [operatorCost, setOperatorCost] = useState<number>(parseInt(bond.amount));

  const theme = useTheme();

  useEffect(() => {
    if (profitMargin === profitMarginPercent && operatorCost === parseInt(bond.amount)) {
      setButtonActive(false);
    } else {
      setButtonActive(true);
    }
  }, [profitMargin, operatorCost]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { value, id } = e.target;
    const numNewValue = parseInt(value) || 0;
    switch (id) {
      case 'profitMargin':
        setProfitMarginPercent(numNewValue.toString());
        break;
      case 'operatorCost':
        setOperatorCost(numNewValue);
        break;
    }
  };

  return (
    <Grid container xs>
      {buttonActive && (
        <Alert
          severity="info"
          sx={{
            width: 1,
            px: 2,
            borderRadius: 0,
            bgcolor: 'background.default',
            color: (theme) => theme.palette.nym.nymWallet.text.blue,
            '& .MuiAlert-icon': { color: (theme) => theme.palette.nym.nymWallet.text.blue, mr: 1 },
          }}
        >
          <strong>Profit margin can be changed once a month, your changes will be applied in the next interval</strong>
        </Alert>
      )}
      <Grid container direction="column">
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3} spacing={1}>
          <Grid item direction="column">
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Profit Margin
            </Typography>
            <Typography
              variant="body1"
              sx={{
                fontSize: 14,
                mb: 2,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Profit margin can be changed once a month
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1} spacing={3}>
              <TextField
                id="profitMargin"
                type="input"
                label="Profit margin"
                value={profitMargin}
                onChange={(e) => handleChange(e)}
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
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3} spacing={1}>
          <Grid item direction="column">
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Operator cost
            </Typography>
            <Typography
              variant="body1"
              sx={{
                fontSize: 14,
                mb: 2,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Lock Wallet after a certain time
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1} spacing={3}>
              <TextField
                id="operatorCost"
                type="input"
                label="Operator cost"
                value={operatorCost}
                onChange={(e) => handleChange(e)}
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
          color: theme.palette.nym.nymWallet.text.blue,
          fontSize: 16,
          textTransform: 'capitalize',
        }}
        subHeaderStyles={{
          m: 0,
        }}
      />
    </Grid>
  );
};
