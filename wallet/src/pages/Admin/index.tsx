import { useContext, useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { Alert, Box, Button, CircularProgress, FormControl, Grid, TextField } from '@mui/material';
import { TauriContractStateParams } from '@src/types';
import { NymCard } from '../../components';
import { getContractParams, setContractParams } from '../../requests';
import { PageLayout } from '../../layouts';
import { AppContext } from '../../context';

const AdminForm: FCWithChildren<{
  params: TauriContractStateParams;
}> = ({ params }) => {
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm({ defaultValues: { ...params } });

  const onSubmit = async (data: TauriContractStateParams) => {
    await setContractParams(data);
  };

  return (
    <FormControl fullWidth>
      <Box sx={{ padding: [3, 5], maxWidth: 700, minWidth: 400 }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <TextField
              {...register('minimum_mixnode_pledge')}
              required
              variant="outlined"
              id="minimum_mixnode_bond"
              name="minimum_mixnode_bond"
              label="Minumum mixnode bond"
              fullWidth
              error={!!errors.minimum_mixnode_pledge}
              helperText={`${errors?.minimum_mixnode_pledge?.amount?.message} ${errors?.minimum_mixnode_pledge?.denom?.message}`}
              InputLabelProps={{ shrink: true }}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('minimum_gateway_pledge')}
              required
              variant="outlined"
              id="minimum_gateway_bond"
              name="minimum_gateway_bond"
              label="Minumum gateway bond"
              fullWidth
              error={!!errors.minimum_gateway_pledge}
              helperText={`${errors?.minimum_gateway_pledge?.amount?.message} ${errors?.minimum_gateway_pledge?.denom?.message}`}
              InputLabelProps={{ shrink: true }}
            />
          </Grid>
        </Grid>
      </Box>
      <Grid
        container
        spacing={1}
        justifyContent="flex-end"
        sx={{
          padding: 2,
        }}
      >
        <Grid item>
          <Button
            onClick={handleSubmit(onSubmit)}
            disabled={isSubmitting}
            variant="contained"
            color="primary"
            type="submit"
            disableElevation
            endIcon={isSubmitting && <CircularProgress size={20} />}
          >
            Update Contract
          </Button>
        </Grid>
      </Grid>
    </FormControl>
  );
};

export const Admin: FCWithChildren = () => {
  const { isAdminAddress, network } = useContext(AppContext);
  const [isLoading, setIsLoading] = useState(false);
  const [params, setParams] = useState<TauriContractStateParams>();

  useEffect(() => {
    const requestContractParams = async () => {
      setIsLoading(true);
      const prms = await getContractParams();
      setParams(prms);
      setIsLoading(false);
    };
    requestContractParams();
  }, [network]);

  if (!isAdminAddress) {
    return (
      <PageLayout>
        <NymCard title="Admin" subheader="Contract administration">
          <Alert severity="error">Sorry, this account is not an admin for {network}</Alert>
        </NymCard>
      </PageLayout>
    );
  }

  return (
    <PageLayout>
      <NymCard title="Admin" subheader="Contract administration" noPadding>
        {isLoading && (
          <Box style={{ display: 'flex', justifyContent: 'center' }}>
            <CircularProgress size={48} />
          </Box>
        )}
        {!isLoading && params && <AdminForm params={params} />}
      </NymCard>
    </PageLayout>
  );
};
