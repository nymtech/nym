import React, { useContext, useEffect, useState } from 'react';
import { Box, Button, FormControl, Grid, Stack, TextField, Typography } from '@mui/material';
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
    <Grid container spacing={2} padding={3}>
      <Grid item sm={12} md={7} lg={8}>
        <Stack direction="column" gap={1}>
          <Typography variant="h6">Change validator</Typography>
          <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
            You can use the validator of your choice by providing its RPC URL address
          </Typography>
          <Stack direction="row" gap={2}>
            <Typography variant="body2">Current selected validator: </Typography>
            <Typography variant="body2">{currentValidatorUrl}</Typography>
          </Stack>
        </Stack>
      </Grid>
      <Grid item sm={12} md={5} lg={4}>
        <Box alignSelf="flex-end">
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
                  disabled={
                    !validatorUrl || validatorUrl.length === 0 || validatorUrl === currentValidatorUrl || isLoading
                  }
                  onClick={saveValidator}
                >
                  Use this validator
                </Button>
              </Stack>
            </FormControl>
          </Stack>
        </Box>
      </Grid>
    </Grid>
  );
};

export default SelectValidator;
