import { boolean, lazy, mixed, number, object, string } from 'yup';
import { isValidHostname, validateKey, validateLocation, validateRawPort, validateVersion } from '../../../utils';
import { NodeType } from '../types';

const nodeSchema = object().shape({
  nodeType: string().required().oneOf(['mixnode', 'gateway']),
  identityKey: string()
    .required('An identity key is required')
    .test('valid-id-key', 'A valid identity key is required', (value) => validateKey(value || '', 32)),

  sphinxKey: string()
    .required('A sphinx key is required')
    .test('valid-sphinx-key', 'A valid sphinx key is required', (value) => validateKey(value || '', 32)),

  signature: string()
    .required('Signature is required')
    .test('valid-signature', 'A valid signature is required', (value) => validateKey(value || '', 64)),

  host: string()
    .required('A host is required')
    .test('valid-host', 'A valid host is required', (value) => (value ? isValidHostname(value) : false)),

  version: string()
    .required('A version is required')
    .test('valid-version', 'A valid version is required', (value) => (value ? validateVersion(value) : false)),

  advancedOpt: boolean().required(),

  location: lazy((value) => {
    if (value) {
      return string()
        .required('A location is required')
        .test('valid-location', 'A valid version is required', (locationValueTest) =>
          locationValueTest ? validateLocation(locationValueTest) : false,
        );
    }
    return mixed().notRequired();
  }),

  mixPort: number()
    .required('A mixport is required')
    .test('valid-mixport', 'A valid mixport is required', (value) => (value ? validateRawPort(value) : false)),

  verlocPort: number()
    .required('A verloc port is required')
    .test('valid-verloc', 'A valid verloc port is required', (value) => (value ? validateRawPort(value) : false)),

  httpApiPort: number()
    .required('A http-api port is required')
    .test('valid-http', 'A valid http-api port is required', (value) => (value ? validateRawPort(value) : false)),

  clientsPort: number()
    .required('A clients port is required')
    .test('valid-clients', 'A valid clients port is required', (value) => (value ? validateRawPort(value) : false)),
});

export default nodeSchema;
