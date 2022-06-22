import { number, object, string } from 'yup';
import { validateAmount } from '../../../utils';

const amountSchema = object().shape({
  amount: object().shape({
    amount: string()
      .required('An amount is required')
      .test('valid-amount', 'Pledge error', async function isValidAmount(this, value) {
        const isValid = await validateAmount(value || '', '100');
        if (!isValid) {
          return this.createError({ message: 'A valid amount is required (min 100)' });
        }
        return true;
      }),
  }),
  profitMargin: number().required('Profit Percentage is required').min(0).max(100),
});

export default amountSchema;
