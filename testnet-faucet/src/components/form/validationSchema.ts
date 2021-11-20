import * as Yup from 'yup'
import { validAmount } from '../../utils'

export const validationSchema = Yup.object().shape({
  address: Yup.string().strict().trim('Cannot have leading space').required(),
  amount: Yup.string()
    .required()
    .test('valid-amount', 'A valid amount is required', (amount) => {
      if (!amount) return false
      return validAmount(amount)
    }),
})
