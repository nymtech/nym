import * as React from 'react';
import { useContext } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { Box, Divider, Stack, Typography } from '@mui/material';
import { DecCoin } from '@nymproject/types';
import { CurrencyInput, TextFieldInput } from '../../components';
import schema from './schema';
import { AppContext } from '../../../../context';
import { TokenPoolSelector } from '../../../../components';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: (bond: DecCoin, signature: string) => void;
  currentBond: DecCoin;
}

interface FormData {
  amount: DecCoin;
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
    <SimpleModal
      open={open}
      onClose={() => {
        reset();
        onClose();
      }}
      onOk={handleSubmit(async (data) => onConfirm(data.amount, data.signature))}
      header="Bond more"
      subHeader="Bond more tokens on your node and receive more rewards"
      okLabel="Next"
      okDisabled={Boolean(errors?.amount || errors?.signature)}
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
              currencyDenom={clientDetails?.display_mix_denom}
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
          <Typography fontWeight={600} textTransform="uppercase">
            {userBalance.balance?.printable_balance || 0}
          </Typography>
        </Stack>
        <Divider sx={{ my: 1 }} />
        <Stack direction="row" justifyContent="space-between">
          <Typography>Current bond</Typography>
          <Typography>{`${currentBond.amount} ${currentBond.denom}`}</Typography>
        </Stack>
        <Divider sx={{ my: 1 }} />
        <Typography fontWeight={400}>Est. fee for this transaction will be cauculated in the next page</Typography>
      </Box>
    </SimpleModal>
  );
};

export default BondModal;
