import React, { useContext, useState } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { clean } from 'semver';
import { Box, Button, Divider, Grid, Stack, TextField, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { isMixnode } from 'src/types';
import { simulateUpdateMixnodeConfig, simulateVestingUpdateMixnodeConfig, updateMixnodeConfig } from 'src/requests';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { bondedInfoParametersValidationSchema } from 'src/components/Bonding/forms/mixnodeValidationSchema';
import { Console } from 'src/utils/console';
import { Alert } from 'src/components/Alert';
import { vestingUpdateMixnodeConfig } from 'src/requests/vesting';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { useGetFee } from 'src/hooks/useGetFee';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { BalanceWarning } from 'src/components/FeeWarning';
import { AppContext } from 'src/context';
import { TBondedMixnode } from 'src/requests/mixnodeDetails';

export const GeneralMixnodeSettings = ({ bondedNode }: { bondedNode: TBondedMixnode }) => {
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const { getFee, fee, resetFeeState } = useGetFee();
  const { userBalance } = useContext(AppContext);

  const theme = useTheme();

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting, isDirty, isValid },
  } = useForm({
    resolver: yupResolver(bondedInfoParametersValidationSchema),
    mode: 'onChange',
    defaultValues: isMixnode(bondedNode) ? bondedNode : {},
  });

  const onSubmit = async (data: {
    host?: string;
    version?: string;
    mixPort?: number;
    verlocPort?: number;
    httpApiPort?: number;
  }) => {
    resetFeeState();
    const { host, version, mixPort, verlocPort, httpApiPort } = data;
    if (host && version && mixPort && verlocPort && httpApiPort) {
      const MixNodeConfigParams = {
        host,
        mix_port: mixPort,
        verloc_port: verlocPort,
        http_api_port: httpApiPort,
        version: clean(version) as string,
      };
      try {
        if (bondedNode.proxy) {
          await vestingUpdateMixnodeConfig(MixNodeConfigParams);
        } else {
          await updateMixnodeConfig(MixNodeConfigParams);
        }
        setOpenConfirmationModal(true);
      } catch (error) {
        Console.error(error);
      }
    }
  };

  return (
    <Grid container xs>
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
          <Stack>
            <Typography fontWeight={600}>
              Changing these values will ONLY change the data about your node on the blockchain.
            </Typography>
            <Typography>Remember to change your node’s config file with the same values too.</Typography>
          </Stack>
        }
        bgColor={`${theme.palette.nym.nymWallet.text.blue}0D !important`}
        dismissable
      />
      <Grid container mt={2}>
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
                {...register('verlocPort')}
                name="verlocPort"
                label="Verloc Port"
                fullWidth
                error={!!errors.verlocPort}
                helperText={errors.verlocPort?.message}
                InputLabelProps={{ shrink: true }}
              />
            </Grid>
            <Grid item width={1}>
              <TextField
                {...register('httpApiPort')}
                name="httpApiPort"
                label="HTTP port"
                fullWidth
                error={!!errors.httpApiPort}
                helperText={errors.httpApiPort?.message}
                InputLabelProps={{ shrink: true }}
              />
            </Grid>
          </Grid>
        </Grid>
        <Divider sx={{ width: '100%' }} />
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
        <Divider sx={{ width: '100%' }} />
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
        <Divider sx={{ width: '100%' }} />
        <Grid item container direction="row" justifyContent="space-between" padding={3}>
          <Grid item />
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Button
              size="large"
              variant="contained"
              disabled={isSubmitting || !isDirty || !isValid}
              onClick={handleSubmit((data) =>
                getFee(bondedNode.proxy ? simulateVestingUpdateMixnodeConfig : simulateUpdateMixnodeConfig, {
                  host: data.host,
                  mix_port: data.mixPort,
                  verloc_port: data.verlocPort,
                  http_api_port: data.httpApiPort,
                  version: data.version,
                }),
              )}
              sx={{ m: 3, mr: 0 }}
              fullWidth
            >
              Submit changes to the blockchain
            </Button>
          </Grid>
        </Grid>
      </Grid>
      <SimpleModal
        open={openConfirmationModal}
        header="Your changes are submitted to the blockchain"
        subHeader="Remember to change the values
        on your node’s config file too."
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
