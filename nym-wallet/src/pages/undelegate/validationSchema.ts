import * as Yup from 'yup';
import { validateKey } from '../../utils';

export const validationSchema = Yup.object().shape({
  identity: Yup.string()
    .required()
    .test('valid-id-key', 'A valid identity key is required', (value) => validateKey(value || '', 32)),
});
