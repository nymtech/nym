import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { Button, Divider, Typography, TextField, Grid, CircularProgress, Box } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { isMixnode } from 'src/types';
import { simulateUpdateGatewayConfig, updateGatewayConfig } from 'src/requests';
import { TBondedGateway } from 'src/context/bonding';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { Console } from 'src/utils/console';
import { Alert } from 'src/components/Alert';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { useGetFee } from 'src/hooks/useGetFee';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { updateGatewayValidationSchema } from 'src/components/Bonding/forms/gatewayValidationSchema';

export const GeneralGatewaySettings = ({ bondedNode }: { bondedNode: TBondedGateway }) => {
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const { getFee, fee, resetFeeState } = useGetFee();

  const theme = useTheme();

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting, isDirty, isValid },
  } = useForm({
    resolver: yupResolver(updateGatewayValidationSchema),
    mode: 'onChange',
    defaultValues: {
      host: bondedNode.host,
      mixPort: bondedNode.mixPort,
      httpApiPort: bondedNode.httpApiPort,
    },
  });

  const onSubmit = async (data: { mixPort?: number; httpApiPort?: number; host?: string }) => {
    resetFeeState();
    const { host, mixPort, httpApiPort } = data;
    try {
      const GatewayConfigParams = {
        host,
        mix_port: mixPort,
        http_api_port: httpApiPort,
      };

      await updateGatewayConfig(GatewayConfigParams);

      setOpenConfirmationModal(true);
    } catch (error) {
      Console.error(error);
    }
  };

  console.log({ isSubmitting, isDirty, isValid, errors });

  return (
    <Grid container xs item>
      {fee && (
        <ConfirmTx
          open
          header="Update node settings"
          fee={fee}
          onConfirm={handleSubmit((d) => onSubmit(d))}
          onPrev={resetFeeState}
          onClose={resetFeeState}
        />
      )}
      {isSubmitting && <LoadingModal />}
      <Alert
        title={
          <Box sx={{ fontWeight: 600 }}>
            Changing these values will ONLY change the data about your node on the blockchain. Remember to change your
            node’s config file with the same values too
          </Box>
        }
        dismissable
      />
      <Grid container>
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Port
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
              <TextField
                {...register('mixPort')}
                name="mixPort"
                label="Mix Port"
                fullWidth
                error={!!errors.mixPort}
                helperText={errors.mixPort?.message}
                InputLabelProps={{ shrink: true }}
              />
            </Grid>

            <Grid item width={1}>
              <TextField
                {...register('httpApiPort')}
                name="httpApiPort"
                label="Client WS API Port"
                fullWidth
                error={!!errors.httpApiPort}
                helperText={errors.httpApiPort?.message}
                InputLabelProps={{ shrink: true }}
              />
            </Grid>
          </Grid>
        </Grid>

        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Host
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
              <TextField
                {...register('host')}
                name="host"
                label="Host"
                fullWidth
                error={!!errors.host}
                helperText={errors.host?.message}
                InputLabelProps={{ shrink: true }}
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
            onClick={handleSubmit((data) =>
              getFee(simulateUpdateGatewayConfig, {
                host: data.host,
                mix_port: data.mixPort,
                http_api_port: data.httpApiPort,
              }),
            )}
            sx={{ m: 3 }}
          >
            Submit changes to the blockchain
          </Button>
        </Grid>
      </Grid>
      <SimpleModal
        open={openConfirmationModal}
        header="Your changes are submitted to the blockchain"
        subHeader="Remember to change the values 
        on your gateways’s config file too."
        okLabel="close"
        hideCloseIcon
        displayInfoIcon
        onOk={async () => {
          await setOpenConfirmationModal(false);
        }}
        buttonFullWidth
        sx={{
          width: '450px',
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
        }}
        subHeaderStyles={{
          width: '100%',
          mb: 1,
          textAlign: 'center',
          color: 'main',
          fontSize: 14,
        }}
      />
    </Grid>
  );
};
