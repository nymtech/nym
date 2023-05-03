import React, { useContext, useEffect, useState } from 'react';
import { Button, FormControl, Stack, TextField } from '@mui/material';
import { useSnackbar } from 'notistack';
import { getSelectedValidatorUrl, setSelectedValidatorUrl } from '../../requests';
import { AppContext } from '../../context';
import { Console } from '../../utils/console';

const SelectValidator = () => {
  const [currentValidatorUrl, setCurrentValidatorUrl] = useState<string>();
  const [validatorUrl, setValidatorUrl] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const { network } = useContext(AppContext);

  const { enqueueSnackbar } = useSnackbar();

  useEffect(() => {
    if (network) {
      getSelectedValidatorUrl(network).then((value) => {
        setCurrentValidatorUrl(value as string | undefined);
        if (value) {
          setValidatorUrl(value);
        } else {
          setValidatorUrl('');
        }
      });
    }
  }, [network]);

  const saveValidator = async () => {
    if (!network || !validatorUrl) {
      return;
    }
    try {
      setIsLoading(true);
      await setSelectedValidatorUrl({ network, url: validatorUrl });
      setCurrentValidatorUrl(validatorUrl);
      enqueueSnackbar('Validator URL saved', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(e as string, { variant: 'error' });
      Console.error(e);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Stack spacing={3} alignItems="center">
      <FormControl fullWidth>
        <Stack spacing={3} mt={2}>
          <TextField
            name="validatorUrl"
            label="Validator URL"
            value={validatorUrl}
            onChange={(e) => setValidatorUrl(e.target.value)}
            error={false}
            InputLabelProps={{ shrink: true }}
            fullWidth
          />
          <Button
            size="large"
            variant="contained"
            disabled={!validatorUrl || validatorUrl.length === 0 || validatorUrl === currentValidatorUrl || isLoading}
            onClick={saveValidator}
          >
            Use this validator
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};

export default SelectValidator;
