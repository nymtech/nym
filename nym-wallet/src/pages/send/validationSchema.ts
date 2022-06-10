import * as Yup from 'yup';
import { validateAmount } from '../../utils';

export const validationSchema = Yup.object().shape({
  to: Yup.string().strict().trim('Cannot have leading space').required(),
  amount: Yup.object().shape({
    amount: Yup.string()
      .required('Amount is required')
      .test('valid-amount', 'A valid amount is required', (amount) => validateAmount(amount || '0', '0')),
    denom: Yup.string(),
  }),
});
