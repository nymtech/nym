import React, { useContext } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { Box, Divider, Stack, Typography } from '@mui/material';
import { AmountData, NodeType } from '../types';
import { AppContext } from '../../../context';
import amountSchema from './amountSchema';
import { SimpleDialog, TokenPoolSelector } from '../../../components';
import { TextFieldInput, CurrencyInput } from '../components';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens } from '../../../utils';

export interface Props {
  nodeType: NodeType;
  open: boolean;
  onClose?: () => void;
  onSubmit: (data: AmountData) => Promise<void>;
}

const AmountModal = ({ open, onClose, onSubmit, nodeType }: Props) => {
  const {
    control,
    setValue,
    setError,
    handleSubmit,
    formState: { errors },
  } = useForm<AmountData>({
    resolver: yupResolver(amountSchema),
    defaultValues: {
      tokenPool: 'balance',
      profitMargin: 10,
    },
  });

  const { userBalance, clientDetails } = useContext(AppContext);

  const onSubmitForm = async (data: AmountData) => {
    if (data.tokenPool === 'balance' && !(await checkHasEnoughFunds(data.amount.amount || ''))) {
      return setError('amount.amount', { message: 'Not enough funds in wallet' });
    }

    if (data.tokenPool === 'locked' && !(await checkHasEnoughLockedTokens(data.amount.amount || ''))) {
      return setError('amount.amount', { message: 'Not enough locked tokens' });
    }
    return onSubmit(data);
  };

  return (
    <SimpleDialog
      open={open}
      onClose={onClose}
      onConfirm={handleSubmit(onSubmitForm)}
      title="Bond"
      subTitle="Step 2/2"
      confirmButton="Next"
      closeButton
    >
      <Box sx={{ mt: 1 }}>
        <form>
          {nodeType === 'mixnode' && (
            <TextFieldInput
              name="profitMargin"
              control={control}
              defaultValue=""
              label="Profit Margin"
              placeholder="Profit Margin"
              error={Boolean(errors.profitMargin)}
              helperText={errors.profitMargin ? errors.profitMargin.message : 'Default is 10%'}
              required
              muiTextFieldProps={{ fullWidth: true }}
              sx={{ mb: 2.5 }}
            />
          )}
          <Stack direction="row" spacing={2}>
            {userBalance.originalVesting && (
              <TokenPoolSelector onSelect={(pool) => setValue('tokenPool', pool)} disabled={false} />
            )}
            <CurrencyInput
              control={control}
              required
              fullWidth
              label="Amount"
              name="amount"
              currencyDenom={clientDetails?.denom}
              errorMessage={errors.amount?.amount?.message}
            />
          </Stack>
        </form>
        <Stack direction="row" justifyContent="space-between" mt={3}>
          <Typography fontWeight={600}>Account balance</Typography>
          <Typography fontWeight={600}>{userBalance.balance?.printable_balance || 0}</Typography>
        </Stack>
        <Divider sx={{ my: 1 }} />
        <Typography fontWeight={400}>Est. fee for this transaction will be cauculated in the next page</Typography>
      </Box>
    </SimpleDialog>
  );
};

export default AmountModal;
