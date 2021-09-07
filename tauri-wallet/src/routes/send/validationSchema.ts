import * as Yup from 'yup'
import { validateAmount, validateKey } from '../../utils'

export const validationSchema = Yup.object().shape({
  to: Yup.string().required(),
  amount: Yup.string()
    .required()
    .test('valid-amount', 'A valid amount is required', (amount) => {
      return validateAmount(amount || '0', '1')
    }),
})
