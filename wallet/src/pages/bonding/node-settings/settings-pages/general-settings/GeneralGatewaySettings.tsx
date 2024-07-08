import React, { useContext, useState } from 'react';
import { useForm } from 'react-hook-form';
import { clean } from 'semver';
import { yupResolver } from '@hookform/resolvers/yup';
import { Button, Divider, Typography, TextField, Grid, Box } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import {
  simulateUpdateGatewayConfig,
  simulateVestingUpdateGatewayConfig,
  updateGatewayConfig,
  vestingUpdateGatewayConfig,
} from '@src/requests';
import { TBondedGateway, useBondingContext } from '@src/context/bonding';
import { SimpleModal } from '@src/components/Modals/SimpleModal';
import { Console } from '@src/utils/console';
import { Alert } from '@src/components/Alert';
import { ConfirmTx } from '@src/components/ConfirmTX';
import { useGetFee } from '@src/hooks/useGetFee';
import { LoadingModal } from '@src/components/Modals/LoadingModal';
import { updateGatewayValidationSchema } from '@src/components/Bonding/forms/gatewayValidationSchema';
import { BalanceWarning } from '@src/components/FeeWarning';
import { AppContext } from '@src/context';

export const GeneralGatewaySettings = ({ bondedNode }: { bondedNode: TBondedGateway }) => {
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const { getFee, fee, resetFeeState } = useGetFee();
  const { refresh } = useBondingContext();
  const { userBalance } = useContext(AppContext);

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
      version: bondedNode.version,
      location: bondedNode.location,
    },
  });

  const onSubmit = async (data: any) => {
    resetFeeState();
    const { host, mixPort, httpApiPort, version, location } = data;

    try {
      const GatewayConfigParams = {
        host,
        mix_port: mixPort,
        location,
        version: clean(version) as string,
        clients_port: httpApiPort,
        verloc_port: bondedNode.verlocPort,
      };

      if (bondedNode.proxy) {
        await vestingUpdateGatewayConfig(GatewayConfigParams, fee?.fee);
      } else {
        await updateGatewayConfig(GatewayConfigParams, fee?.fee);
      }

      setOpenConfirmationModal(true);
    } catch (error) {
      Console.error(error);
    }
  };

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
        >
          {fee.amount?.amount && userBalance?.balance?.amount.amount && (
            <Box sx={{ mb: 2 }}>
              <BalanceWarning fee={fee.amount.amount} />
            </Box>
          )}
        </ConfirmTx>
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
                label="Client Port"
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
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Version
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
              <TextField
                {...register('version')}
                name="version"
                label="Version"
                fullWidth
                error={!!errors.version}
                helperText={errors.version?.message}
                InputLabelProps={{ shrink: true }}
              />
            </Grid>
          </Grid>
        </Grid>
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Location
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
              <TextField
                {...register('location')}
                name="location"
                label="Location"
                fullWidth
                error={!!errors.location}
                helperText={errors.location?.message}
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
              getFee(bondedNode.proxy ? simulateVestingUpdateGatewayConfig : simulateUpdateGatewayConfig, {
                host: data.host,
                mix_port: data.mixPort,
                clients_port: data.httpApiPort,
                location: bondedNode.location!,
                version: data.version,
                verloc_port: bondedNode.verlocPort,
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
          setOpenConfirmationModal(false);
          await refresh();
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
