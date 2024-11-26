import * as Yup from 'yup';
import { TauriContractStateParams } from 'src/types';
import { isLessThan, isGreaterThan, validateAmount } from 'src/utils';

const operatingCostAndPmValidation = (params?: TauriContractStateParams) => {
  const defaultParams = {
    profit_margin_percent: {
      minimum: parseFloat(params?.profit_margin.minimum || '20%'),
      maximum: parseFloat(params?.profit_margin.maximum || '50%'),
    },

    interval_operating_cost: {
      minimum: parseFloat(params?.operating_cost.minimum.amount || '0'),
      maximum: parseFloat(params?.operating_cost.maximum.amount || '1000000000'),
    },
  };

  return {
    profit_margin_percent: Yup.number()
      .required('Profit Percentage is required')
      .min(defaultParams.profit_margin_percent.minimum)
      .max(defaultParams.profit_margin_percent.maximum),
    interval_operating_cost: Yup.object().shape({
      amount: Yup.string()
        .required('An operating cost is required')
        // eslint-disable-next-line prefer-arrow-callback
        .test('valid-operating-cost', 'A valid amount is required', async function isValidAmount(this, value) {
          if (
            value &&
            (!Number(value) ||
              isLessThan(+value, defaultParams.interval_operating_cost.minimum) ||
              isGreaterThan(+value, Number(defaultParams.interval_operating_cost.maximum)))
          ) {
            return this.createError({
              message: `A valid amount is required (min ${defaultParams?.interval_operating_cost.minimum} - max ${defaultParams?.interval_operating_cost.maximum})`,
            });
          }
          return true;
        }),
    }),
  };
};

export const nymNodeAmountSchema = (params?: TauriContractStateParams) =>
  Yup.object().shape({
    pledge: Yup.object().shape({
      amount: Yup.string()
        .required('An amount is required')
        .test('valid-amount', 'Pledge error', async function isValidAmount(this, value) {
          const isValid = await validateAmount(value || '', '100');
          if (!isValid) {
            return this.createError({ message: 'A valid amount is required (min 100)' });
          }
          return true;
        }),
    }),
    ...operatingCostAndPmValidation(params),
  });
