import * as Yup from 'yup'
import {
  isValidHostname,
  validateAmount,
  validateKey,
  validateLocation,
  validateRawPort,
  validateVersion,
} from '../../utils'

export const validationSchema = Yup.object().shape({
  identityKey: Yup.string()
    .required('An indentity key is required')
    .test('valid-id-key', 'A valid identity key is required', function (value) {
      return validateKey(value || '')
    }),
  sphinxKey: Yup.string()
    .required('A sphinx key is required')
    .test(
      'valid-sphinx-key',
      'A valid sphinx key is required',
      function (value) {
        return validateKey(value || '')
      }
    ),
  amount: Yup.string()
    .required('An amount is required')
    .test(
      'valid-amount',
      'A valid amount is required (min 100 punks)',
      function (value) {
        return validateAmount(value || '', '100000000')
        // minimum amount needs to come from the backend - replace when available
      }
    ),

  host: Yup.string()
    .required('A host is required')
    .test('valid-host', 'A valid host is required', function (value) {
      return !!value ? isValidHostname(value) : false
    }),
  version: Yup.string()
    .required('A version is required')
    .test('valid-version', 'A valid version is required', function (value) {
      return !!value ? validateVersion(value) : false
    }),
  location: Yup.lazy((value) => {
    if (!!value) {
      return Yup.string()
        .required('A location is required')
        .test(
          'valid-location',
          'A valid version is required',
          function (value) {
            return !!value ? validateLocation(value) : false
          }
        )
    }
    return Yup.mixed().notRequired()
  }),
  mixPort: Yup.number()
    .required('A mixport is required')
    .test('valid-mixport', 'A valid mixport is required', function (value) {
      return !!value ? validateRawPort(value) : false
    }),
  verlocPort: Yup.number()
    .required('A verloc port is required')
    .test('valid-verloc', 'A valid verloc port is required', function (value) {
      return !!value ? validateRawPort(value) : false
    }),
  httpApiPort: Yup.number()
    .required('A http-api port is required')
    .test('valid-http', 'A valid http-api port is required', function (value) {
      return !!value ? validateRawPort(value) : false
    }),
  clientsPort: Yup.number()
    .required('A clients port is required')
    .test(
      'valid-clients',
      'A valid clients port is required',
      function (value) {
        return !!value ? validateRawPort(value) : false
      }
    ),
})
