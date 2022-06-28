import { object, string } from 'yup';
import { validateAmount, validateKey } from '../../../../utils';

const schema = object().shape({
  amount: object().shape({
    amount: string()
      .required('An amount is required')
      .test('valid-amount', 'Pledge error', async function isValidAmount(this, value) {
        const isValid = await validateAmount(value || '', '0.01');
        if (!isValid) {
          return this.createError({ message: 'A valid amount is required (min 0.01)' });
        }
        return true;
      }),
  }),
  signature: string()
    .required('Signature is required')
    .test('valid-signature', 'A valid signature is required', (value) => validateKey(value || '', 64)),
});

export default schema;
