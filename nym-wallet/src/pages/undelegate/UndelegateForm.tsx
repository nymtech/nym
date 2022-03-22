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
import { addHours, format } from 'date-fns';
import { validationSchema } from './validationSchema';
import { EnumNodeType, PendingUndelegate, TDelegation } from '../../types';
import { undelegate, vestingUnelegateFromMixnode } from '../../requests';
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
  onError,
  onSuccess,
}: {
  delegations?: TDelegation[];
  pendingUndelegations?: PendingUndelegate[];
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
    let res;
    try {
      res = await undelegate({
        type: data.nodeType,
        identity: data.identity,
      });

      if (!res) {
        res = await vestingUnelegateFromMixnode(data.identity);
      }

      if (!res) {
        onError('An error occurred when undelegating');
        return;
      }

      onSuccess(`Successfully requested undelegation from ${res.target_address}`);
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
                            ? `Pending - Expected time of completion: ${format(addHours(new Date(), 1), 'HH:00')}`
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
