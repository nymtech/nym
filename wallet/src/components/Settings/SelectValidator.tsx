import { useContext, useEffect, useState } from 'react';
import { Button, FormControl, Grid, Stack, Switch, TextField, Typography } from '@mui/material';
import { useSnackbar } from 'notistack';
import {
  checkMixnodeOwnership,
  getDefaultValidatorUrl,
  getSelectedValidatorUrl,
  resetValidatorUrl,
  setSelectedValidatorUrl as setSelectedValidatorUrlReq,
} from '../../requests';
import { AppContext } from '../../context';
import { Console } from '../../utils/console';
import { Network } from '../../types';

const SelectValidator = () => {
  const [customValidatorEnabled, setCustomValidatorEnabled] = useState<boolean>(false);
  const [selectedValidatorUrl, setSelectedValidatorUrl] = useState<string | null>();
  const [defaultValidatorUrl, setDefaultValidatorUrl] = useState<string | null>();
  const [validatorUrlInput, setValidatorUrlInput] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const { network } = useContext(AppContext);

  const { enqueueSnackbar } = useSnackbar();

  const getDefaultValidator = async (net: Network) => {
    if (!network) {
      return;
    }
    try {
      const defaultValidator = await getDefaultValidatorUrl(net);
      setDefaultValidatorUrl(defaultValidator);
    } catch (e) {
      Console.error(`an error occurred while requesting the default validator URL: ${e}`);
    }
  };

  const getSelectedValidator = async (net: Network) => {
    if (!network) {
      return null;
    }
    try {
      const selectedValidator = await getSelectedValidatorUrl(net);
      setSelectedValidatorUrl(selectedValidator);
    } catch (e) {
      Console.error(`an error occurred while requesting the selected validator URL: ${e}`);
    }
    return null;
  };

  useEffect(() => {
    if (network) {
      getDefaultValidator(network);
      getSelectedValidator(network);
    }
  }, [network, customValidatorEnabled]);

  useEffect(() => {
    // on network change, turn off the custom val switch if there is no selected val
    // for this network
    if (!selectedValidatorUrl) {
      setCustomValidatorEnabled(false);
      setValidatorUrlInput('');
    }
  }, [network, selectedValidatorUrl]);

  useEffect(() => {
    if (selectedValidatorUrl && selectedValidatorUrl !== defaultValidatorUrl) {
      setCustomValidatorEnabled(true);
    }

    if (selectedValidatorUrl) {
      setValidatorUrlInput(selectedValidatorUrl);
    }
  }, [selectedValidatorUrl, defaultValidatorUrl, network]);

  const onToggle = async () => {
    if (!customValidatorEnabled) {
      setCustomValidatorEnabled(true);
      return;
    }
    setIsLoading(true);
    try {
      await resetValidatorUrl(network as Network);
      setValidatorUrlInput('');
      setSelectedValidatorUrl(null);
      setCustomValidatorEnabled(false);
    } catch (e) {
      Console.error(e);
    } finally {
      setIsLoading(false);
    }
  };

  const saveValidator = async () => {
    if (!network || !validatorUrlInput || validatorUrlInput === defaultValidatorUrl) {
      return;
    }
    setIsLoading(true);
    try {
      // this tauri request also does a basic connection check
      await setSelectedValidatorUrlReq({ network, url: validatorUrlInput });
    } catch (e) {
      Console.error(e);
      enqueueSnackbar(`Invalid validator URL: ${e}`, { variant: 'error' });
      setIsLoading(false);
      return;
    }

    // to enforce the validator URL is valid, try to query the node ownership
    // if it fails, that means the endpoint is wrong
    // TODO this check logic should be handled directly in the rust side, `select_nyxd_url` command
    try {
      await checkMixnodeOwnership();
      enqueueSnackbar('Validator URL saved', { variant: 'success' });
    } catch (e) {
      Console.error(e);
      enqueueSnackbar('The given validator URL is not valid for the currently selected network', { variant: 'error' });
      await resetValidatorUrl(network as Network);
      setSelectedValidatorUrl(null);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Grid container spacing={2} padding={3}>
      <Grid item sm={12} md={7} lg={8}>
        <Stack direction="column" gap={1}>
          <Typography variant="h6">Change validator</Typography>
          <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
            You can use the validator of your choice by providing its RPC URL address
          </Typography>
          <Stack direction="row" spacing={3} mt={2} alignItems="center">
            <Typography>Turn Off</Typography>
            <Switch checked={customValidatorEnabled} onChange={onToggle} inputProps={{ 'aria-label': 'controlled' }} />
            <Typography>Turn On</Typography>
          </Stack>
        </Stack>
      </Grid>
      <Grid item sm={12} md={5} lg={4}>
        <Stack spacing={3} alignItems="flex-end">
          {customValidatorEnabled ? (
            <FormControl fullWidth>
              <Stack spacing={3} mt={2}>
                <TextField
                  name="validatorUrl"
                  label="Validator URL"
                  value={validatorUrlInput}
                  onChange={(e) => setValidatorUrlInput(e.target.value)}
                  error={false}
                  InputLabelProps={{ shrink: true }}
                  fullWidth
                  disabled={!customValidatorEnabled}
                  autoFocus
                />
                <Button
                  size="large"
                  variant="contained"
                  disabled={
                    !validatorUrlInput ||
                    validatorUrlInput.length === 0 ||
                    validatorUrlInput === defaultValidatorUrl ||
                    validatorUrlInput === selectedValidatorUrl ||
                    isLoading ||
                    !customValidatorEnabled
                  }
                  onClick={saveValidator}
                >
                  Use this validator
                </Button>
              </Stack>
            </FormControl>
          ) : (
            <Stack spacing={2} alignItems="end" mt={3} mr={1}>
              <Typography variant="body2">Default validator address</Typography>
              <Typography>{defaultValidatorUrl}</Typography>
            </Stack>
          )}
        </Stack>
      </Grid>
    </Grid>
  );
};

export default SelectValidator;
