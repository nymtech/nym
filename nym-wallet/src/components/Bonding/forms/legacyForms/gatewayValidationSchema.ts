import * as Yup from 'yup';
import {
  isLessThan,
  isValidHostname,
  validateAmount,
  validateKey,
  validateLocation,
  validateRawPort,
  validateVersion,
} from 'src/utils';

export const gatewayValidationSchema = Yup.object().shape({
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

  location: Yup.string()
    .required('A location is required')
    .test('valid-location', 'A valid location is required', (locationValueTest) =>
      locationValueTest ? validateLocation(locationValueTest) : false,
    ),

  mixPort: Yup.number()
    .required('A mixport is required')
    .test('valid-mixport', 'A valid mixport is required', (value) => (value ? validateRawPort(value) : false)),

  clientsPort: Yup.number()
    .required('A clients port is required')
    .test('valid-clients', 'A valid clients port is required', (value) => (value ? validateRawPort(value) : false)),
});

export const amountSchema = Yup.object().shape({
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
  operatorCost: Yup.object().shape({
    amount: Yup.string()
      .required('An operating cost is required')
      // eslint-disable-next-line
      .test('valid-operating-cost', 'A valid amount is required (min 40)', async function isValidAmount(this, value) {
        if (value && (!Number(value) || isLessThan(+value, 40))) {
          return false;
        }

        return true;
      }),
  }),
});

export const updateGatewayValidationSchema = Yup.object().shape({
  host: Yup.string()
    .required('A host is required')
    .test('no-whitespace', 'Host cannot contain whitespace', (value) => !/\s/.test(value || ''))
    .test('valid-host', 'A valid host is required', (value) => (value ? isValidHostname(value) : false)),

  mixPort: Yup.number()
    .required('A mixport is required')
    .test('valid-mixport', 'A valid mixport is required', (value) => (value ? validateRawPort(value) : false)),

  httpApiPort: Yup.number()
    .required('A clients port is required')
    .test('valid-clients', 'A valid clients port is required', (value) => (value ? validateRawPort(value) : false)),
  location: Yup.string().test('valid-location', 'A valid location is required', (value) =>
    value ? validateLocation(value) : false,
  ),
  version: Yup.string()
    .test('no-whitespace', 'A version cannot contain whitespace', (value) => !/\s/.test(value || ''))
    .test('valid-version', 'A valid version is required', (value) => (value ? validateVersion(value) : false)),
});
