import * as Yup from 'yup';
import {
  isGreaterThan,
  isLessThan,
  isValidHostname,
  validateAmount,
  validateKey,
  validateRawPort,
  validateVersion,
} from 'src/utils';
import { TauriContractStateParams } from '../../../types';

export const mixnodeValidationSchema = Yup.object().shape({
  identityKey: Yup.string()
    .required('An identity key is required')
    .test('valid-id-key', 'A valid identity key is required', (value) => validateKey(value || '', 32)),

  sphinxKey: Yup.string()
    .required('A sphinx key is required')
    .test('valid-sphinx-key', 'A valid sphinx key is required', (value) => validateKey(value || '', 32)),

  host: Yup.string()
    .required('A host is required')
    .test('no-whitespace', 'Host cannot contain whitespace', (value) => !/\s/.test(value || ''))
    .test('valid-host', 'A valid host is required', (value) => (value ? isValidHostname(value) : false)),

  version: Yup.string()
    .required('A version is required')
    .test('no-whitespace', 'A version cannot contain whitespace', (value) => !/\s/.test(value || ''))
    .test('valid-version', 'A valid version is required', (value) => (value ? validateVersion(value) : false)),

  mixPort: Yup.number()
    .required('A mixport is required')
    .test('valid-mixport', 'A valid mixport is required', (value) => (value ? validateRawPort(value) : false)),

  verlocPort: Yup.number()
    .required('A verloc port is required')
    .test('valid-verloc', 'A valid verloc port is required', (value) => (value ? validateRawPort(value) : false)),

  httpApiPort: Yup.number()
    .required('A http-api port is required')
    .test('valid-http', 'A valid http-api port is required', (value) => (value ? validateRawPort(value) : false)),
});

const operatingCostAndPmValidation = (params?: TauriContractStateParams) => {
  const defaultParams = {
    profit_margin: {
      minimum: parseFloat(params?.profit_margin.minimum || '0%'),
      maximum: parseFloat(params?.profit_margin.maximum || '100%'),
    },

    operating_cost: {
      minimum: parseFloat(params?.operating_cost.minimum.amount || '0'),
      maximum: parseFloat(params?.operating_cost.maximum.amount || '1000000000'),
    },
  };

  return {
    profitMargin: Yup.number()
      .required('Profit Percentage is required')
      .min(defaultParams.profit_margin.minimum)
      .max(defaultParams.profit_margin.maximum),
    operatorCost: Yup.object().shape({
      amount: Yup.string()
        .required('An operating cost is required')
        // eslint-disable-next-line prefer-arrow-callback
        .test('valid-operating-cost', 'A valid amount is required', async function isValidAmount(this, value) {
          if (
            value &&
            (!Number(value) ||
              isLessThan(+value, defaultParams.operating_cost.minimum) ||
              isGreaterThan(+value, Number(defaultParams.operating_cost.maximum)))
          ) {
            return this.createError({
              message: `A valid amount is required (min ${defaultParams?.operating_cost.minimum} - max ${defaultParams?.operating_cost.maximum})`,
            });
          }
          return true;
        }),
    }),
  };
};

export const amountSchema = (params?: TauriContractStateParams) =>
  Yup.object().shape({
    amount: Yup.object().shape({
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

export const bondedInfoParametersValidationSchema = Yup.object().shape({
  host: Yup.string()
    .required('A host is required')
    .test('no-whitespace', 'Host cannot contain whitespace', (value) => !/\s/.test(value || ''))
    .test('valid-host', 'A valid host is required', (value) => (value ? isValidHostname(value) : false)),

  version: Yup.string()
    .required('A version is required')
    .test('no-whitespace', 'A version cannot contain whitespace', (value) => !/\s/.test(value || ''))
    .test('valid-version', 'A valid version is required', (value) => (value ? validateVersion(value) : false)),

  mixPort: Yup.number()
    .required('A mixport is required')
    .test('valid-mixport', 'A valid mixport is required', (value) => (value ? validateRawPort(value) : false)),

  verlocPort: Yup.number()
    .required('A verloc port is required')
    .test('valid-verloc', 'A valid verloc port is required', (value) => (value ? validateRawPort(value) : false)),

  httpApiPort: Yup.number()
    .required('A http-api port is required')
    .test('valid-http', 'A valid http-api port is required', (value) => (value ? validateRawPort(value) : false)),
});

export const bondedNodeParametersValidationSchema = (params?: TauriContractStateParams) =>
  Yup.object().shape({
    ...operatingCostAndPmValidation(params),
  });
