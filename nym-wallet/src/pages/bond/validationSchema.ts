import * as Yup from 'yup';
import {
  isValidHostname,
  validateAmount,
  validateKey,
  validateLocation,
  validateRawPort,
  validateVersion,
} from '../../utils';

export const validationSchema = Yup.object().shape({
  identityKey: Yup.string()
    .required('An indentity key is required')
    .test('valid-id-key', 'A valid identity key is required', (value) => validateKey(value || '', 32)),

  sphinxKey: Yup.string()
    .required('A sphinx key is required')
    .test('valid-sphinx-key', 'A valid sphinx key is required', (value) => validateKey(value || '', 32)),

  ownerSignature: Yup.string()
    .required('Signature is required')
    .test('valid-signature', 'A valid signature is required', (value) => validateKey(value || '', 64)),

  profitMarginPercent: Yup.number().required('Profit Percentage is required').min(0).max(100),

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

  host: Yup.string()
    .required('A host is required')
    .test('valid-host', 'A valid host is required', (value) => (value ? isValidHostname(value) : false)),

  version: Yup.string()
    .required('A version is required')
    .test('valid-version', 'A valid version is required', (value) => (value ? validateVersion(value) : false)),

  location: Yup.lazy((value) => {
    if (value) {
      return Yup.string()
        .required('A location is required')
        .test('valid-location', 'A valid version is required', (locationValueTest) =>
          locationValueTest ? validateLocation(locationValueTest) : false,
        );
    }
    return Yup.mixed().notRequired();
  }),

  mixPort: Yup.number()
    .required('A mixport is required')
    .test('valid-mixport', 'A valid mixport is required', (value) => (value ? validateRawPort(value) : false)),

  verlocPort: Yup.number()
    .required('A verloc port is required')
    .test('valid-verloc', 'A valid verloc port is required', (value) => (value ? validateRawPort(value) : false)),

  httpApiPort: Yup.number()
    .required('A http-api port is required')
    .test('valid-http', 'A valid http-api port is required', (value) => (value ? validateRawPort(value) : false)),

  clientsPort: Yup.number()
    .required('A clients port is required')
    .test('valid-clients', 'A valid clients port is required', (value) => (value ? validateRawPort(value) : false)),
});
