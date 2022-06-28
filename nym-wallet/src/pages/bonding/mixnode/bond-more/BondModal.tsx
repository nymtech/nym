import * as React from 'react';
import { useContext } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { Box, Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { CurrencyInput, SimpleDialog, TextFieldInput } from '../../components';
import schema from './schema';
import { AppContext } from '../../../../context';
import { TokenPoolSelector } from '../../../../components';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: (bond: MajorCurrencyAmount, signature: string) => void;
  currentBond: MajorCurrencyAmount;
}

interface FormData {
  amount: MajorCurrencyAmount;
  tokenPool: string;
  signature: string;
}

const BondModal = ({ open, onClose, onConfirm, currentBond }: Props) => {
  const {
    control,
    handleSubmit,
    reset,
    setValue,
    formState: { errors },
  } = useForm<FormData>({
    resolver: yupResolver(schema),
  });

  const { userBalance, clientDetails } = useContext(AppContext);

  return (
    <SimpleDialog
      open={open}
      onClose={() => {
        reset();
        onClose();
      }}
      onConfirm={handleSubmit(async (data) => onConfirm(data.amount, data.signature))}
      title="Bond more"
      subTitle="Bond more tokens on your node and receive more rewards"
      confirmButton="Next"
      closeButton
      disabled={Boolean(errors?.amount || errors?.signature)}
    >
      <Box sx={{ mt: 1 }}>
        <form>
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
          <TextFieldInput
            name="signature"
            control={control}
            defaultValue=""
            label="Signature"
            placeholder="Signature"
            error={Boolean(errors.signature)}
            helperText={errors.signature?.message}
            required
            muiTextFieldProps={{ fullWidth: true }}
            sx={{ mt: 2.5 }}
          />
        </form>
        <Stack direction="row" justifyContent="space-between" mt={3}>
          <Typography fontWeight={600}>Account balance</Typography>
          <Typography fontWeight={600}>{userBalance.balance?.printable_balance || 0}</Typography>
        </Stack>
        <Divider sx={{ my: 1 }} />
        <Stack direction="row" justifyContent="space-between">
          <Typography>Current bond</Typography>
          <Typography>{`${currentBond.amount} ${currentBond.denom}`}</Typography>
        </Stack>
        <Divider sx={{ my: 1 }} />
        <Typography fontWeight={400}>Est. fee for this transaction will be cauculated in the next page</Typography>
      </Box>
    </SimpleDialog>
  );
};

export default BondModal;
