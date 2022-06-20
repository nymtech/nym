import React from 'react';
import { useForm, Controller } from 'react-hook-form';
import {
  ListItem,
  ListItemText,
  Box,
  Autocomplete,
  Button,
  CircularProgress,
  FormControl,
  Grid,
  TextField,
} from '@mui/material';
import { yupResolver } from '@hookform/resolvers/yup';
import { format } from 'date-fns';
import { EnumNodeType, PendingUndelegate, TDelegation } from '@nymproject/types';
import { validationSchema } from './validationSchema';
import { undelegateFromMixnode, vestingUndelegateFromMixnode } from '../../requests';
import { Fee } from '../../components';

type TFormData = {
  nodeType: EnumNodeType;
  identity: string;
};

const defaultValues = {
  nodeType: EnumNodeType.mixnode,
  identity: '',
};

export const UndelegateForm = ({
  delegations,
  pendingUndelegations,
  currentEndEpoch,
  onError,
  onSuccess,
}: {
  delegations?: TDelegation[];
  pendingUndelegations?: PendingUndelegate[];
  currentEndEpoch?: BigInt;
  onError: (message?: string) => void;
  onSuccess: (message?: string) => void;
}) => {
  const {
    control,
    handleSubmit,
    setValue,
    formState: { errors, isSubmitting },
  } = useForm<TFormData>({
    defaultValues,
    resolver: yupResolver(validationSchema),
  });

  const onSubmit = async (data: TFormData) => {
    const delegation = (delegations || []).find((d) => d.node_identity === data.identity);

    if (!delegation) {
      onError(`Could not undelegate from ${data.identity} as not found in list of delegations for this account`);
      return;
    }

    let res;
    try {
      if ((delegation.proxy || '').trim().length === 0) {
        // the owner of the delegation is main account (the owner of the vesting account), so it is delegation with unlocked tokens
        res = await undelegateFromMixnode(data.identity);
      } else {
        // the delegation is with locked tokens, so use the vesting contract
        res = await vestingUndelegateFromMixnode(data.identity);
      }

      if (!res) {
        onError('An error occurred when undelegating');
        return;
      }

      onSuccess(`Successfully requested undelegation from ${data.identity}. Tx hash = ${res.transaction_hash}`);
    } catch (e) {
      onError(e as string);
    }
  };

  return (
    <FormControl fullWidth>
      <Box sx={{ p: 3 }}>
        <Grid container spacing={3} direction="column">
          <Grid item xs={12}>
            <Controller
              control={control}
              name="identity"
              render={() => (
                <Autocomplete
                  disabled={isSubmitting}
                  getOptionDisabled={(opt) => pendingUndelegations?.some((item) => item.mix_identity === opt) || false}
                  options={delegations?.map((d) => d.node_identity) || []}
                  renderOption={(props, opt) => (
                    <ListItem
                      {...props}
                      onClick={(e: React.MouseEvent<HTMLLIElement>) => {
                        setValue('identity', opt);
                        props.onClick!(e);
                      }}
                      disablePadding
                      disableGutters
                    >
                      <ListItemText
                        primary={opt}
                        secondary={
                          pendingUndelegations?.some((item) => item.mix_identity === opt)
                            ? `Pending - Expected time of completion: ${
                                currentEndEpoch ? format(new Date(Number(currentEndEpoch) * 1000), 'HH:mm') : 'N/A'
                              }`
                            : undefined
                        }
                      />
                    </ListItem>
                  )}
                  renderInput={(params) => (
                    <TextField
                      {...params}
                      required
                      variant="outlined"
                      id="identity"
                      name="identity"
                      label="Mixnode identity"
                      error={!!errors.identity}
                      helperText={errors.identity?.message}
                      fullWidth
                    />
                  )}
                />
              )}
            />
          </Grid>
          <Grid item xs={12}>
            <Fee feeType="UndelegateFromMixnode" />
          </Grid>
        </Grid>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          p: 3,
          pt: 0,
        }}
      >
        <Button
          onClick={handleSubmit(onSubmit)}
          variant="contained"
          color="primary"
          type="submit"
          data-testid="submit-button"
          disableElevation
          disabled={isSubmitting}
          endIcon={isSubmitting && <CircularProgress size={20} />}
          size="large"
        >
          Undelegate stake
        </Button>
      </Box>
    </FormControl>
  );
};
