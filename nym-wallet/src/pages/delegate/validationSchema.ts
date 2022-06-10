import * as Yup from 'yup';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens, validateAmount, validateKey } from '../../utils';

export const validationSchema = Yup.object().shape({
  identity: Yup.string()
    .required()
    .test(
      'valid-id-key',
      'A valid identity key is required e.g. 824WyExLUWvLE2mpSHBatN4AoByuLzfnHFeHWiBYzg4z',
      (value) => (value ? validateKey(value, 32) : false),
    ),

  amount: Yup.object().shape({
    amount: Yup.string()
      .required()
      .test('valid-amount', 'A valid amount is required', async function isValidAmount(value) {
        const isValid = await validateAmount(value || '', '0');

        if (!isValid) {
          return this.createError({ message: 'A valid amount is required' });
        }

        const hasEnoughBalance = await checkHasEnoughFunds(value || '');
        const hasEnoughLocked = await checkHasEnoughLockedTokens(value || '');

        if (this.parent.tokenPool === 'balance' && !hasEnoughBalance) {
          return this.createError({ message: 'Not enough funds in wallet' });
        }

        if (this.parent.tokenPool === 'locked' && !hasEnoughLocked) {
          return this.createError({ message: 'Not enough locked tokens' });
        }

        return true;
      }),
    denom: Yup.string().required(),
  }),
});
