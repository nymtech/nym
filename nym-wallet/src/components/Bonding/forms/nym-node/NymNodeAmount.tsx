import { Stack, TextField, Box, FormHelperText } from '@mui/material';
import { useForm } from 'react-hook-form';
import { TBondNymNodeArgs } from 'src/types';
import { yupResolver } from '@hookform/resolvers/yup';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { nymNodeAmountSchema } from './amountValidationSchema';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { checkHasEnoughFunds } from 'src/utils';

const defaultNymNodeCostParamValues: TBondNymNodeArgs['costParams'] = {
  profit_margin_percent: '10',
  interval_operating_cost: { amount: '40', denom: 'nym' },
};

const defaultNymNodePledgeValue: TBondNymNodeArgs['pledge'] = {
  amount: '100',
  denom: 'nym',
};

type NymNodeDataProps = {
  onClose: () => void;
  onBack: () => void;
  onNext: () => Promise<void>;
  step: number;
};

const NymNodeAmount = ({ onClose, onBack, onNext, step }: NymNodeDataProps) => {
  const {
    formState: { errors },
    register,
    getValues,
    setValue,
    setError,
    handleSubmit,
  } = useForm({
    mode: 'all',
    defaultValues: {
      pledge: defaultNymNodePledgeValue,
      ...defaultNymNodeCostParamValues,
    },
    resolver: yupResolver(nymNodeAmountSchema()),
  });

  console.log(errors, 'errors');

  const handleRequestValidation = async () => {
    const values = getValues();

    const hasSufficientTokens = await checkHasEnoughFunds(values.pledge.amount);

    if (hasSufficientTokens) {
      handleSubmit(onNext)();
    } else {
      setError('pledge.amount', { message: 'Not enough tokens' });
    }
  };

  return (
    <SimpleModal
      open
      onOk={handleRequestValidation}
      onClose={onClose}
      header="Bond Nym Node"
      subHeader={`Step ${step}/3`}
      okLabel="Next"
      onBack={onBack}
      okDisabled={Object.keys(errors).length > 0}
    >
      <Stack gap={3}>
        <CurrencyFormField
          required
          fullWidth
          label="Amount"
          autoFocus
          onChanged={(newValue) => {
            setValue('pledge.amount', newValue.amount, { shouldValidate: true });
          }}
          validationError={errors.pledge?.amount?.message}
          denom={defaultNymNodePledgeValue.denom}
          initialValue={defaultNymNodePledgeValue.amount}
        />

        <Box>
          <CurrencyFormField
            required
            fullWidth
            label="Operating cost"
            onChanged={(newValue) => {
              setValue('interval_operating_cost', newValue, { shouldValidate: true });
            }}
            validationError={errors.interval_operating_cost?.amount?.message}
            denom={defaultNymNodeCostParamValues.interval_operating_cost.denom}
            initialValue={defaultNymNodeCostParamValues.interval_operating_cost.amount}
          />
          <FormHelperText>
            Monthly operational costs of running your node. If your node is in the active set the amount will be paid
            back to you from the rewards.
          </FormHelperText>
        </Box>
        <Box>
          <TextField
            {...register('profit_margin_percent')}
            name="profit_margin_percent"
            label="Profit margin"
            error={Boolean(errors.profit_margin_percent)}
            helperText={errors.profit_margin_percent?.message}
            fullWidth
          />
          <FormHelperText>
            The percentage of node rewards that you as the node operator take before rewards are distributed to operator
            and delegators.
          </FormHelperText>
        </Box>
      </Stack>
    </SimpleModal>
  );
};

export default NymNodeAmount;
