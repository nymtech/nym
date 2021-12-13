import * as Yup from 'yup'
import { validateAmount, validateKey } from '../../utils'

export const validationSchema = Yup.object().shape({
  identity: Yup.string()
    .required()
    .test(
      'valid-id-key',
      'A valid identity key is required e.g. 824WyExLUWvLE2mpSHBatN4AoByuLzfnHFeHWiBYzg4z',
      (value) => (!!value ? validateKey(value, 32) : false),
    ),
  amount: Yup.string()
    .required()
    .test('valid-amount-key', 'A valid amount is required', (value) => (!!value ? validateAmount(value, '0') : false)),
})
