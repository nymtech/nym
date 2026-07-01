import React, { useContext, useEffect, useState } from 'react';
import { Box, Button, Stack, TextField, Typography } from '@mui/material';
import { alpha, Theme } from '@mui/material/styles';
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

const RpcEndpointPanel = ({ label, url }: { label: string; url: string | null | undefined }) => (
  <Box
    sx={{
      p: 2,
      borderRadius: 2,
      border: (t: Theme) => `1px solid ${t.palette.divider}`,
      bgcolor: (t: Theme) =>
        t.palette.mode === 'dark' ? alpha(t.palette.common.white, 0.04) : alpha(t.palette.common.black, 0.02),
      width: '100%',
    }}
  >
    <Typography
      variant="caption"
      color="text.secondary"
      sx={{ textTransform: 'uppercase', letterSpacing: 0.5, display: 'block', mb: 0.5 }}
    >
      {label}
    </Typography>
    <Typography variant="body2" sx={{ fontFamily: 'monospace', wordBreak: 'break-all', lineHeight: 1.5 }}>
      {url ?? '…'}
    </Typography>
  </Box>
);

const SelectValidator = () => {
  const [isEditing, setIsEditing] = useState(false);
  const [selectedValidatorUrl, setSelectedValidatorUrl] = useState<string | null>();
  const [defaultValidatorUrl, setDefaultValidatorUrl] = useState<string | null>();
  const [validatorUrlInput, setValidatorUrlInput] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const { network } = useContext(AppContext);

  const { enqueueSnackbar } = useSnackbar();

  const usingCustom = Boolean(
    selectedValidatorUrl && defaultValidatorUrl && selectedValidatorUrl !== defaultValidatorUrl,
  );
  const activeUrl = usingCustom ? selectedValidatorUrl : defaultValidatorUrl;

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
  }, [network]);

  useEffect(() => {
    if (!selectedValidatorUrl) {
      setValidatorUrlInput('');
      setIsEditing(false);
    }
  }, [network, selectedValidatorUrl]);

  useEffect(() => {
    if (selectedValidatorUrl) {
      setValidatorUrlInput(selectedValidatorUrl);
    }
  }, [selectedValidatorUrl]);

  const openEditor = () => {
    setValidatorUrlInput(selectedValidatorUrl ?? defaultValidatorUrl ?? '');
    setIsEditing(true);
  };

  const cancelEditing = () => {
    setValidatorUrlInput(selectedValidatorUrl ?? '');
    setIsEditing(false);
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
      setSelectedValidatorUrl(validatorUrlInput);
      setIsEditing(false);
    } catch (e) {
      Console.error(e);
      enqueueSnackbar('The given validator URL is not valid for the currently selected network', { variant: 'error' });
      await resetValidatorUrl(network as Network);
      setSelectedValidatorUrl(null);
    } finally {
      setIsLoading(false);
    }
  };

  const canSave =
    validatorUrlInput.length > 0 &&
    validatorUrlInput !== defaultValidatorUrl &&
    validatorUrlInput !== selectedValidatorUrl &&
    !isLoading;

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="flex-start" padding={3} gap={2}>
        <Stack direction="column" gap={1} sx={{ minWidth: 0 }}>
          <Typography variant="h6">Change validator</Typography>
          <Typography variant="body2" sx={{ color: 'text.secondary', lineHeight: 1.5 }}>
            The wallet talks to the chain through an RPC endpoint. The default is recommended for most users.
          </Typography>
        </Stack>
        <Box sx={{ flexShrink: 0, alignSelf: 'flex-end' }}>
          {isEditing ? (
            <Button variant="text" disabled={isLoading} onClick={cancelEditing}>
              Cancel
            </Button>
          ) : (
            <Button variant="text" disabled={isLoading} onClick={openEditor}>
              Use custom RPC URL
            </Button>
          )}
        </Box>
      </Stack>

      <Box sx={{ px: 3, pb: 3 }}>
        {isEditing ? (
          <Stack spacing={2} sx={{ maxWidth: 560 }}>
            <TextField
              name="validatorUrl"
              label="Validator URL"
              placeholder="https://"
              value={validatorUrlInput}
              onChange={(e) => setValidatorUrlInput(e.target.value)}
              InputLabelProps={{ shrink: true }}
              fullWidth
              size="small"
              disabled={isLoading}
              autoFocus
            />
            <Box>
              <Button variant="contained" size="medium" disabled={!canSave} onClick={saveValidator}>
                Save
              </Button>
            </Box>
          </Stack>
        ) : (
          <RpcEndpointPanel label={usingCustom ? 'Custom RPC endpoint' : 'Default RPC endpoint'} url={activeUrl} />
        )}
      </Box>
    </Box>
  );
};

export default SelectValidator;
