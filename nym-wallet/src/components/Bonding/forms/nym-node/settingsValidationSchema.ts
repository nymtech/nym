import { isValidHostname, validateRawPort } from 'src/utils';
import * as Yup from 'yup';

const settingsValidationSchema = Yup.object().shape({
  host: Yup.string()
    .required('A host is required')
    .test('no-whitespace', 'Host cannot contain whitespace', (value) => !/\s/.test(value || ''))
    .test('valid-host', 'A valid host is required', (value) => (value ? isValidHostname(value) : false)),

  custom_http_port: Yup.number()
    .nullable()
    .transform((numberVal, stringVal) => {
      if (stringVal === '') {
        return null;
      }
      if (!Number(stringVal)) {
        return stringVal;
      }
      return numberVal;
    })
    .test('valid-http', 'A valid http port is required', (value) => {
      if (value === null) {
        return true;
      }
      return value ? validateRawPort(value) : false;
    }),
});

export { settingsValidationSchema };
