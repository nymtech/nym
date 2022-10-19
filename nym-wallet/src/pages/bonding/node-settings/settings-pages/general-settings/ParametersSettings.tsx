import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import {
  Button,
  Divider,
  Typography,
  TextField,
  InputAdornment,
  Grid,
  Alert,
  IconButton,
  CircularProgress,
  Box,
} from '@mui/material';
import { useTheme } from '@mui/material/styles';
import CloseIcon from '@mui/icons-material/Close';
import { isMixnode } from 'src/types';
import { updateMixnodeCostParams } from 'src/requests';
import { TBondedMixnode, TBondedGateway } from 'src/context/bonding';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { bondedNodeParametersValidationSchema } from 'src/components/Bonding/forms/mixnodeValidationSchema';
import { Console } from 'src/utils/console';
import { decimalToFloatApproximation, decimalToPercentage } from '@nymproject/types';

export const ParametersSettings = ({ bondedNode }: { bondedNode: TBondedMixnode | TBondedGateway }): JSX.Element => {
  const [open, setOpen] = useState(true);
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);

  const theme = useTheme();

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting, isDirty, isValid },
  } = useForm({
    resolver: yupResolver(bondedNodeParametersValidationSchema),
    mode: 'onChange',
    defaultValues: isMixnode(bondedNode)
      ? {
          operatorCost: bondedNode.operatorCost,
          profitMargin: bondedNode.profitMargin,
        }
      : {},
  });

  const onSubmit = async (data: { operatorCost?: string; profitMargin?: string }) => {
    if (data.operatorCost && data.profitMargin) {
      const MixNodeCostParams = {
        profit_margin_percent: (+data.profitMargin / 100).toString(),
        interval_operating_cost: {
          denom: bondedNode.bond.denom,
          amount: data.operatorCost.toString(),
        },
      };
      try {
        await updateMixnodeCostParams(MixNodeCostParams);
        setOpenConfirmationModal(true);
      } catch (error) {
        Console.error(error);
      }
    }
  };

  return (
    <Grid container xs item>
      {open && (
        <Alert
          severity="info"
          action={
            <IconButton
              aria-label="close"
              color="inherit"
              size="small"
              onClick={() => {
                setOpen(false);
              }}
            >
              <CloseIcon fontSize="inherit" />
            </IconButton>
          }
          sx={{
            width: 1,
            px: 2,
            borderRadius: 0,
            bgcolor: 'background.default',
            color: (theme) => theme.palette.nym.nymWallet.text.blue,
            '& .MuiAlert-icon': { color: (theme) => theme.palette.nym.nymWallet.text.blue, mr: 1 },
          }}
        >
          <Box sx={{ fontWeight: 600 }}>
            Profit margin can be changed once a month, your changes will be applied in the next interval
          </Box>
        </Alert>
      )}
      <Grid container direction="column">
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3} spacing={1}>
          <Grid item>
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
		    Changes to PM will be applied in the next interval.
            </Typography>
          </Grid>
          <Grid spacing={3} container item alignItems="center" sm={12} md={6}>
            {isMixnode(bondedNode) && (
              <Grid item width={1}>
                <TextField
                  {...register('profitMargin')}
                  name="profitMargin"
                  label="Profit margin"
                  fullWidth
                  error={!!errors.profitMargin}
                  helperText={errors.profitMargin?.message}
                  InputProps={{
                    endAdornment: (
                      <InputAdornment position="end">
                        <Box>%</Box>
                      </InputAdornment>
                    ),
                  }}
                />
              </Grid>
            )}
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3} spacing={1}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Operating cost
            </Typography>
            <Typography
              variant="body1"
              sx={{
                fontSize: 14,
                mb: 2,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
            Changes to cost will be applied in the next interval.
	    </Typography>
          </Grid>
          <Grid spacing={3} container item alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
              <TextField
                {...register('operatorCost')}
                name="operatorCost"
                label="Operating cost"
                fullWidth
                error={!!errors.operatorCost}
                helperText={errors?.operatorCost?.message}
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <Box>{bondedNode.bond.denom.toUpperCase()}</Box>
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
            disabled={isSubmitting || !isDirty || !isValid}
            onClick={handleSubmit((d) => onSubmit(d))}
            type="submit"
            sx={{ m: 3, width: '320px' }}
            endIcon={isSubmitting && <CircularProgress size={20} />}
          >
            Save all display changes
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
